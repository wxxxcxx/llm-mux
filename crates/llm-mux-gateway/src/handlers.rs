use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response, Sse};
use llm_mux_core::codec::{Authenticator, Codec, RouteInfo};
use llm_mux_core::ir::IrRequest;
use llm_mux_core::types::Protocol;
use llm_mux_core::{ConfigAuthenticator, ConfigurableRouter, Router};
use reqwest::Client;
use tracing::error;

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ConfigurableRouter>,
    pub authenticator: Arc<ConfigAuthenticator>,
}

fn validate_auth(auth: &Arc<ConfigAuthenticator>, api_key: &Option<String>) -> Option<Response> {
    match api_key {
        Some(key) if auth.authenticate(key).is_ok() => None,
        _ => Some(
            (StatusCode::UNAUTHORIZED,
             r#"{"error":{"message":"Invalid or missing API key","type":"authentication_error"}}"#)
                .into_response(),
        ),
    }
}

fn outbound_codec_for(protocol: Protocol) -> Option<Box<dyn Codec>> {
    match protocol {
        Protocol::OpenAiChat => Some(Box::new(openai_chat_codec::ChatCompletionsCodec)),
        Protocol::Anthropic => Some(Box::new(anthropic_codec::MessagesCodec)),
        Protocol::OpenAiResponses => Some(Box::new(openai_responses_codec::ResponsesCodec)),
    }
}

pub async fn health() -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/json")],
        r#"{"status":"ok"}"#,
    )
        .into_response()
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let api_key = extract_api_key_chat(&headers);
    if let Some(resp) = validate_auth(&state.authenticator, &api_key) {
        return resp;
    }
    let inbound = openai_chat_codec::ChatCompletionsCodec;
    handle_request(state, api_key, Protocol::OpenAiChat, &inbound, &body).await
}

pub async fn messages(State(state): State<AppState>, headers: HeaderMap, body: String) -> Response {
    let api_key = extract_api_key_anthropic(&headers);
    if let Some(resp) = validate_auth(&state.authenticator, &api_key) {
        return resp;
    }
    let inbound = anthropic_codec::MessagesCodec;
    handle_request(state, api_key, Protocol::Anthropic, &inbound, &body).await
}

pub async fn responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let api_key = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-api-key").and_then(|v| v.to_str().ok()))
        .map(|s| s.to_string());
    if let Some(resp) = validate_auth(&state.authenticator, &api_key) {
        return resp;
    }
    let inbound = openai_responses_codec::ResponsesCodec;
    handle_request(state, api_key, Protocol::OpenAiResponses, &inbound, &body).await
}

fn extract_api_key_chat(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn extract_api_key_anthropic(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .or_else(|| headers.get("Authorization"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ").or_else(|| {
                if !v.starts_with("Bearer ") {
                    Some(v)
                } else {
                    None
                }
            })
        })
        .map(|s| s.to_string())
}

async fn handle_request(
    state: AppState,
    api_key: Option<String>,
    inbound_protocol: Protocol,
    inbound_codec: &dyn Codec,
    body: &str,
) -> Response {
    let ir = match inbound_codec.decode_request(body.as_bytes()) {
        Ok(ir) => ir,
        Err(e) => {
            return codec_error_response(inbound_codec, 400, &e);
        }
    };

    let is_stream = ir.stream.unwrap_or(false);

    if is_stream {
        return handle_stream_request(state, api_key, inbound_protocol, ir, body).await;
    }

    let route_info = RouteInfo {
        request_id: uuid::Uuid::now_v7().to_string(),
        model: ir.model.clone(),
        inbound_protocol,
        stream: false,
        has_tools: ir.has_tools(),
        has_media: ir.has_media(),
        api_key: api_key.clone(),
    };

    let route = match state.router.route(&route_info) {
        Ok(r) => r,
        Err(e) => {
            return codec_error_response(inbound_codec, 502, &e);
        }
    };

    let outbound_codec = match outbound_codec_for(route.protocol) {
        Some(c) => c,
        None => {
            return codec_error_response(
                inbound_codec,
                502,
                &format!("unsupported outbound protocol: {:?}", route.protocol),
            );
        }
    };

    let mut outbound_ir = ir.clone();
    outbound_ir.model = route.model.clone();

    let outbound_body = match outbound_codec.encode_request(&outbound_ir) {
        Ok(b) => b,
        Err(e) => {
            return codec_error_response(inbound_codec, 500, &e);
        }
    };

    let client = Client::new();
    let url = upstream_url(&route);

    let mut req_builder = client.post(&url).body(outbound_body);

    for (k, v) in &route.headers {
        req_builder = req_builder.header(k.as_str(), v.as_str());
    }
    req_builder = req_builder.header("Authorization", format!("Bearer {}", route.api_key));
    req_builder = req_builder.header("Content-Type", "application/json");

    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("upstream request failed: {}", e);
            return codec_error_response(inbound_codec, 502, &e);
        }
    };

    let status = upstream_resp.status();
    let resp_bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return codec_error_response(inbound_codec, 502, &e);
        }
    };

    match outbound_codec.decode_response(&resp_bytes) {
        Ok(ir_resp) => match inbound_codec.encode_response(&ir_resp) {
            Ok(encoded) => {
                let mut resp = Response::new(axum::body::Body::from(encoded));
                *resp.status_mut() = StatusCode::OK;
                resp.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/json"),
                );
                resp
            }
            Err(e) => codec_error_response(inbound_codec, 500, &e),
        },
        Err(e) => {
            error!("failed to decode upstream response: {}", e);
            if status.is_server_error() || status.is_client_error() {
                let code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
                (code, axum::body::Body::from(resp_bytes)).into_response()
            } else {
                codec_error_response(inbound_codec, 502, &e)
            }
        }
    }
}

async fn handle_stream_request(
    state: AppState,
    api_key: Option<String>,
    inbound_protocol: Protocol,
    ir: IrRequest,
    _body: &str,
) -> Response {
    let route_info = RouteInfo {
        request_id: uuid::Uuid::now_v7().to_string(),
        model: ir.model.clone(),
        inbound_protocol,
        stream: true,
        has_tools: ir.has_tools(),
        has_media: ir.has_media(),
        api_key,
    };

    let route = match state.router.route(&route_info) {
        Ok(r) => r,
        Err(e) => {
            let inbound = outbound_codec_for(inbound_protocol)
                .unwrap_or_else(|| Box::new(openai_chat_codec::ChatCompletionsCodec));
            return codec_error_response(inbound.as_ref(), 502, &e);
        }
    };

    let inbound_codec = match outbound_codec_for(inbound_protocol) {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":{"message":"unsupported inbound protocol"}}"#,
            )
                .into_response();
        }
    };

    let outbound_codec = match outbound_codec_for(route.protocol) {
        Some(c) => c,
        None => {
            return codec_error_response(
                inbound_codec.as_ref(),
                502,
                &format!("unsupported outbound protocol: {:?}", route.protocol),
            );
        }
    };

    let mut outbound_ir = ir.clone();
    outbound_ir.model = route.model.clone();

    let outbound_body = match outbound_codec.encode_request(&outbound_ir) {
        Ok(b) => b,
        Err(e) => {
            return codec_error_response(inbound_codec.as_ref(), 500, &e);
        }
    };

    let client = Client::new();
    let url = upstream_url(&route);

    let mut req_builder = client.post(&url).body(outbound_body);

    for (k, v) in &route.headers {
        req_builder = req_builder.header(k.as_str(), v.as_str());
    }
    req_builder = req_builder.header("Authorization", format!("Bearer {}", route.api_key));
    req_builder = req_builder.header("Content-Type", "application/json");

    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("upstream stream request failed: {}", e);
            return codec_error_response(inbound_codec.as_ref(), 502, &e);
        }
    };

    if !upstream_resp.status().is_success() {
        let status = upstream_resp.status();
        let err_body = match upstream_resp.bytes().await {
            Ok(b) => b,
            Err(e) => return codec_error_response(inbound_codec.as_ref(), 502, &e),
        };
        let code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        return (code, axum::body::Body::from(err_body)).into_response();
    }

    let stream = crate::sse::sse_proxy(upstream_resp, inbound_codec, outbound_codec);

    Sse::new(stream).into_response()
}

fn upstream_url(route: &llm_mux_core::codec::RouteResult) -> String {
    match route.protocol {
        Protocol::OpenAiChat => format!("{}/v1/chat/completions", route.base_url),
        Protocol::Anthropic => format!("{}/v1/messages", route.base_url),
        Protocol::OpenAiResponses => format!("{}/v1/responses", route.base_url),
    }
}

fn codec_error_response(
    codec: &dyn Codec,
    code: u16,
    msg: &(impl std::fmt::Display + ?Sized),
) -> Response {
    let err_body = codec.write_error(code, &msg.to_string());
    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::body::Body::from(err_body)).into_response()
}

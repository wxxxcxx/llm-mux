pub mod genai_bridge;

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response, Sse};
use genai::Client as GenaiClient;
use llm_mux_core::codec::{Authenticator, Codec, RouteInfo};
use llm_mux_core::ir::IrRequest;
use llm_mux_core::types::{
    ContentType, Protocol, Role,
};
use llm_mux_core::{ConfigAuthenticator, ConfigurableRouter, Router};
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ConfigurableRouter>,
    pub authenticator: Arc<ConfigAuthenticator>,
    pub genai: Arc<GenaiClient>,
}

fn validate_auth(auth: &Arc<ConfigAuthenticator>, api_key: &Option<String>) -> Option<Response> {
    match api_key {
        Some(key) if auth.authenticate(key).is_ok() => None,
        _ => {
            debug!(has_key = api_key.is_some(), "auth rejected");
            Some(
                (StatusCode::UNAUTHORIZED,
                 r#"{"error":{"message":"Invalid or missing API key","type":"authentication_error"}}"#)
                    .into_response(),
            )
        }
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
    let body_preview: String = body.chars().take(500).collect();
    debug!(
        protocol = ?inbound_protocol,
        body_len = body.len(),
        body_preview = %body_preview,
        "incoming request"
    );

    let ir = match inbound_codec.decode_request(body.as_bytes()) {
        Ok(ir) => {
            info!(
                model = %ir.model,
                stream = ir.stream,
                msg_count = ir.messages.len(),
                has_tools = ir.has_tools(),
                has_media = ir.has_media(),
                "decoded request"
            );
            ir
        }
        Err(e) => {
            error!(error = %e, "failed to decode request");
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
        Ok(r) => {
            info!(
                target_protocol = ?r.protocol,
                target_model = %r.model,
                base_url = %r.base_url,
                "route matched"
            );
            r
        }
        Err(e) => {
            error!(error = %e, "routing failed");
            return codec_error_response(inbound_codec, 502, &e);
        }
    };

    // genai Client 进行下游调用
    let mut outbound_ir = ir.clone();
    outbound_ir.model = route.model.clone();

    let chat_messages: Vec<_> = outbound_ir
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                llm_mux_core::types::Role::System => genai::chat::ChatRole::System,
                llm_mux_core::types::Role::User => genai::chat::ChatRole::User,
                llm_mux_core::types::Role::Assistant => genai::chat::ChatRole::Assistant,
                llm_mux_core::types::Role::Tool => genai::chat::ChatRole::Tool,
            };
            let text: String = m
                .content
                .iter()
                .filter_map(|b| {
                    if b.content_type == llm_mux_core::types::ContentType::Text {
                        b.text.as_ref().map(|t| t.text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            genai::chat::ChatMessage::new(role, text)
        })
        .collect();

    // 构造 genai ChatRequest
    let mut chat_req = genai::chat::ChatRequest::new(chat_messages);

    // 传递系统提示
    if !outbound_ir.system_prompt.is_empty() {
        let sys_text: String = outbound_ir
            .system_prompt
            .iter()
            .filter_map(|b| {
                if b.content_type == llm_mux_core::types::ContentType::Text {
                    b.text.as_ref().map(|t| t.text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !sys_text.is_empty() {
            chat_req = chat_req.with_system(sys_text);
        }
    }

    // 传递采样参数 + 推理配置
    let mut chat_options = genai::chat::ChatOptions::default();
    if let Some(t) = outbound_ir.temperature {
        chat_options = chat_options.with_temperature(t);
    }
    if let Some(n) = outbound_ir.max_tokens {
        chat_options = chat_options.with_max_tokens(n as u32);
    }
    if let Some(p) = outbound_ir.top_p {
        chat_options = chat_options.with_top_p(p);
    }
    if let Some(ref fc) = outbound_ir.response_format {
        if fc.format_type == "json_schema" {
            if let Some(ref schema) = fc.json_schema {
                chat_options =
                    chat_options.with_response_format(genai::chat::ChatResponseFormat::JsonSpec(
                        genai::chat::JsonSpec::new("response", schema.clone()),
                    ));
            }
        } else if fc.format_type == "json_object" {
            chat_options =
                chat_options.with_response_format(genai::chat::ChatResponseFormat::JsonMode);
        }
    }
    if let Some(ref thinking) = outbound_ir.thinking {
        if let Some(ref mode) = thinking.mode {
            if mode == "enabled" {
                if let Some(budget) = thinking.budget_tokens {
                    chat_options = chat_options
                        .with_reasoning_effort(genai::chat::ReasoningEffort::Budget(budget as u32));
                }
            }
        }
    }
    // 透传 genai 不支持的参数
    let mut extra = serde_json::Map::new();
    if let Some(k) = outbound_ir.top_k {
        extra.insert("top_k".into(), serde_json::Value::from(k));
    }
    if !outbound_ir.stop_sequences.is_empty() {
        extra.insert(
            "stop_sequences".into(),
            serde_json::Value::Array(
                outbound_ir
                    .stop_sequences
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    if !extra.is_empty() {
        chat_options = chat_options.with_extra_body(serde_json::Value::Object(extra));
    }

    // 使用 genai 命名空间语法路由
    let genai_model = match route.format {
        genai::adapter::AdapterKind::OpenCodeGo => format!("opencode_go::{}", route.model),
        kind => format!("{}::{}", kind, route.model),
    };

    match state
        .genai
        .exec_chat(
            genai::ModelSpec::from_name(genai_model.clone()),
            chat_req,
            Some(&chat_options),
        )
        .await
    {
        Ok(genai_resp) => {
            let ir_resp = ir_from_genai_response(&genai_resp, &outbound_ir.model);
            match inbound_codec.encode_response(&ir_resp) {
                Ok(encoded) => (StatusCode::OK, axum::body::Body::from(encoded)).into_response(),
                Err(e) => codec_error_response(inbound_codec, 500, &e),
            }
        }
        Err(e) => {
            error!(error = %e, "genai upstream call failed");
            codec_error_response(inbound_codec, 502, &e)
        }
    }
}

fn ir_from_genai_response(
    genai_resp: &genai::chat::ChatResponse,
    model: &str,
) -> llm_mux_core::ir::IrResponse {
    crate::handlers::genai_bridge::ir_from_genai_response(genai_resp, model)
}

async fn handle_stream_request(
    state: AppState,
    api_key: Option<String>,
    inbound_protocol: Protocol,
    mut ir: IrRequest,
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
        None => return (StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"error":{"message":"unsupported inbound protocol"}}"#).into_response(),
    };

    // 构建 genai ChatMessages (复用非流式路径逻辑)
    ir.model = route.model.clone();
    let chat_messages: Vec<_> = ir.messages.iter()
        .map(|m| {
            let role = match m.role {
                Role::System => genai::chat::ChatRole::System,
                Role::User => genai::chat::ChatRole::User,
                Role::Assistant => genai::chat::ChatRole::Assistant,
                Role::Tool => genai::chat::ChatRole::Tool,
            };
            let text: String = m.content.iter()
                .filter_map(|b| if b.content_type == ContentType::Text { b.text.as_ref().map(|t| t.text.clone()) } else { None })
                .collect::<Vec<_>>().join("");
            genai::chat::ChatMessage::new(role, text)
        }).collect();

    let chat_req = genai::chat::ChatRequest::new(chat_messages);
    let mut opts = genai::chat::ChatOptions::default();
    if let Some(t) = ir.temperature { opts = opts.with_temperature(t); }
    if let Some(n) = ir.max_tokens { opts = opts.with_max_tokens(n as u32); }

    let genai_model = match route.format {
        genai::adapter::AdapterKind::OpenCodeGo => format!("opencode_go::{}", route.model),
        kind => format!("{}::{}", kind, route.model),
    };

    match state.genai.exec_chat_stream(genai::ModelSpec::from_name(&genai_model), chat_req, Some(&opts)).await {
        Ok(stream_resp) => {
            let stream = genai_stream_to_sse(stream_resp.stream);
            Sse::new(stream).into_response()
        }
        Err(e) => {
            error!(error = %e, "genai stream call failed");
            codec_error_response(inbound_codec.as_ref(), 502, &e)
        }
    }
}

fn genai_stream_to_sse(
    mut chat_stream: genai::chat::ChatStream,
) -> tokio_stream::wrappers::UnboundedReceiverStream<Result<axum::response::sse::Event, std::convert::Infallible>> {
    use tokio::sync::mpsc;
    use tokio_stream::StreamExt;
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(event) = StreamExt::next(&mut chat_stream).await {
            match event {
                Ok(genai::chat::ChatStreamEvent::Chunk(chunk)) => {
                    let data = serde_json::json!({
                        "choices": [{"delta": {"content": &chunk.content}, "index": 0}],
                        "object": "chat.completion.chunk"
                    });
                    let _ = tx.send(Ok(axum::response::sse::Event::default()
                        .data(serde_json::to_string(&data).unwrap_or_default())));
                }
                Ok(genai::chat::ChatStreamEvent::End(_)) => {
                    let _ = tx.send(Ok(axum::response::sse::Event::default().data("[DONE]")));
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    let _ = tx.send(Ok(axum::response::sse::Event::default()
                        .data(format!("{{\"error\":\"{}\"}}", e))));
                    break;
                }
            }
        }
    });
    tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
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

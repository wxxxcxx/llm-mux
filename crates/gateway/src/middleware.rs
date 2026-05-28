//! 请求 ID 中间件。

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::Instrument;
use uuid::Uuid;

pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = Uuid::now_v7().to_string();

    req.extensions_mut().insert(RequestId(request_id.clone()));

    let span = tracing::info_span!("request", request_id = %request_id);

    let mut response = async move { next.run(req).await }.instrument(span).await;

    if let Ok(v) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("X-Request-ID", v);
    }

    response
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

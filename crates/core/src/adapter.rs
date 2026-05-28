use genai::chat::{ChatRequest, ChatResponse, ChatStreamEvent};

use super::types::Protocol;

/// 适配器层统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("decode error: {0}")]
    Decode(String),

    #[error("encode error: {0}")]
    Encode(String),

    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// 入站协议适配器：将外部协议的 HTTP 请求/响应翻译为 genai IR。
pub trait Adapter: Send + Sync {
    /// 返回此适配器处理的入站协议。
    fn protocol(&self) -> Protocol;

    /// 将外部协议的 HTTP 请求体解码为 genai ChatRequest。
    fn decode_request(&self, body: &[u8]) -> Result<ChatRequest, AdapterError>;

    /// 将 genai ChatResponse 编码为外部协议的 HTTP 响应体。
    fn encode_response(&self, response: &ChatResponse) -> Result<Vec<u8>, AdapterError>;

    /// 将 genai ChatStreamEvent 编码为外部协议的 SSE 数据行。
    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError>;

    /// 将适配器错误编码为该入站协议的 HTTP 错误响应体。
    fn encode_error(&self, error: &AdapterError) -> Vec<u8>;
}

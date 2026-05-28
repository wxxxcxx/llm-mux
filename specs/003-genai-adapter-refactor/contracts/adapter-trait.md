# Adapter trait 合约

**功能**: 基于 genai 适配器模式重构 | **日期**: 2026-05-28

---

## 1. Adapter trait 定义

```rust
use genai::chat::{ChatMessage, ChatRequest, ChatResponse, ChatStreamEvent};
use genai::ModelIden;

/// 入站协议适配器：将外部协议的 HTTP 请求/响应翻译为 genai IR。
pub trait Adapter: Send + Sync {
    /// 返回此适配器处理的入站协议。
    fn protocol(&self) -> Protocol;

    /// 将外部协议的 HTTP 请求体解码为 genai ChatRequest。
    ///
    /// # Errors
    /// 返回 `AdapterError::Decode` 如果请求体不是合法的目标协议格式。
    fn decode_request(&self, body: &[u8]) -> Result<ChatRequest, AdapterError>;

    /// 将 genai ChatResponse 编码为外部协议的 HTTP 响应体。
    ///
    /// # Errors
    /// 返回 `AdapterError::Encode` 如果响应内容无法编码为协议格式。
    fn encode_response(&self, response: &ChatResponse) -> Result<Vec<u8>, AdapterError>;

    /// 将 genai ChatStreamEvent 编码为外部协议的 SSE 数据行（不含 `data:` 前缀和换行）。
    ///
    /// 流开始: 适配器自行决定是否需要发送初始事件。
    /// 流结束: `ChatStreamEvent::End(_)` 触发协议标准的结束标记。
    ///
    /// # Errors
    /// 返回 `AdapterError::Encode` 事件无法编码为协议格式。
    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError>;

    /// 将适配器错误编码为该入站协议的 HTTP 错误响应体。
    fn encode_error(&self, error: &AdapterError) -> Vec<u8>;
}
```

## 2. AdapterError 定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("decode error: {0}")]
    Decode(String),

    #[error("encode error: {0}")]
    Encode(String),

    #[error("unsupported: {0}")]
    Unsupported(String),
}
```

## 3. 适配器实现约定

### 3.1 decode_request 约定

1. 输入 `body` 为原始 JSON 字节流
2. 反序列化为目标协议的原生请求结构（如 `CreateChatCompletionRequest`）
3. 提取所有 genai `ChatRequest` 所需字段（messages、tools 等）
4. 提取所有 genai `ChatOptions` 所需字段（temperature、max_tokens 等）
5. 对 genai ChatOptions 未覆盖但协议支持的字段，写入 `ChatOptions.extra_body`
6. 对 genai `ContentPart` 变体未覆盖但协议支持的内容类型，映射为 `ContentPart::Custom`

### 3.2 encode_response 约定

1. 输入 `response` 为 genai 的 `ChatResponse`
2. 遍历 `response.content` 的 `ContentPart`，映射回目标协议的响应内容格式
3. 对于 `ContentPart::Custom(part)`：
   - 检查 `part.model_iden` 判断原始来源
   - 如果能识别为同协议的原始内容，直接还原
   - 如果不能识别，降级为文本或跳过
4. 构造 target protocol 的完整 JSON 响应体

### 3.3 encode_stream_event 约定

1. 流开始前：适配器可发送协议的初始 SSE 事件（如 Anthropic `message_start`）
2. `Chunk(content)` → 文本增量 SSE 事件
3. `ReasoningChunk(content)` → 推理增量 SSE 事件
4. `ToolCallChunk(tool)` → 工具调用增量 SSE 事件
5. `End(stream_end)` → 流结束（必要时发送 `[DONE]` 或 `message_stop`）

### 3.4 错误处理约定

1. 解码失败：返回 `AdapterError::Decode`，网关返回 400
2. 编码失败：返回 `AdapterError::Encode`，网关返回 502
3. `AdapterError::Unsupported`：用于协议不支持的功能请求

## 4. Protocol 枚举

```rust
/// 入站协议标识 — 通过 HTTP 端点路径自动推断
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// /v1/chat/completions
    OpenAiChat,
    /// /v1/responses
    OpenAiResponses,
    /// /v1/messages
    Anthropic,
}
```

## 5. 与现有 Codec trait 的差异

| 维度 | 现有 Codec | 新 Adapter |
|---|---|---|
| 下游请求构造 | `encode_request()` — 适配器负责构造 HTTP 请求体 | **删除** — genai Client 负责 |
| 下游响应解析 | `decode_response()` — 适配器负责解析 HTTP 响应 | **删除** — genai Client 负责 |
| 下游流解析 | `decode_stream_event()` — 适配器负责逐事件解析 SSE | **删除** — genai streamer 负责 |
| known_fields | HashMap 用于透传未知字段 | **删除** — `extra_body` 替代 |
| 双向编解码 | 适配器同时负责入站和出站的双向翻译 | **入站 only** — 下游由 genai 全权处理 |
| 返回类型 | 自建 `IrRequest`/`IrResponse`/`IrStreamEvent` | genai 类型 |

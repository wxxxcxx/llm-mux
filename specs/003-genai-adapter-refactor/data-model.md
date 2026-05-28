# 数据模型: 基于 genai 适配器模式重构

**日期**: 2026-05-28 | **来源**: [plan.md](./plan.md) Phase 1

> 以下描述重构后的目标数据模型。标注 ✅ 表示 genai 已有，🔄 表示重命名/重构，🆕 表示新增，❌ 表示删除。

---

## IR 层 (genai 类型体系)

以下类型全部来自 genai crate，LLM Mux 不再自定义：

### ChatRequest — 统一请求 ✅ genai

| 字段 | 类型 | 说明 |
|---|---|---|
| `messages` | `Vec<ChatMessage>` | 对话消息列表 |
| `tools` | `Vec<Tool>` | 可用工具定义 |
| `previous_response_id` | `Option<String>` | Responses 会话 ID |
| `store` | `Option<bool>` | 是否存储响应 |

### ChatResponse — 统一非流式响应 ✅ genai

| 字段 | 类型 | 说明 |
|---|---|---|
| `content` | `MessageContent` | 响应内容（ContentPart 序列） |
| `reasoning_content` | `Option<String>` | 推理/思考内容 |
| `model_iden` | `ModelIden` | 模型标识 |
| `stop_reason` | `Option<StopReason>` | 停止原因 |
| `usage` | `Usage` | Token 用量 |
| `response_id` | `Option<String>` | Responses 会话 ID |
| `captured_raw_body` | `Option<Value>` | 原始响应体 |

### ChatStreamEvent — 统一流事件 ✅ genai

| 变体 | 说明 |
|---|---|
| `Start` | 流开始 |
| `Chunk(StreamChunk)` | 文本增量 `{ content: String }` |
| `ReasoningChunk(StreamChunk)` | 推理增量 |
| `ToolCallChunk(ToolChunk)` | 工具调用增量 `{ tool_call: ToolCall }` |
| `End(StreamEnd)` | 流结束（携带 captured_usage, captured_stop_reason, captured_content 等） |

### ChatOptions — 请求参数 ✅ genai

| 字段 | 类型 | genai 支持 |
|---|---|---|
| `temperature` | `Option<f64>` | ✅ 全部 |
| `max_tokens` | `Option<u32>` | ✅ 全部 |
| `top_p` | `Option<f64>` | ✅ 全部 |
| `stop_sequences` | `Vec<String>` | ✅ 全部（Responses 除外） |
| `reasoning_effort` | `Option<ReasoningEffort>` | ✅ 全部 |
| `response_format` | `Option<ChatResponseFormat>` | ✅ 全部 |
| `tool_choice` | `Option<ToolChoice>` | ✅ 全部 |
| `seed` | `Option<u64>` | ✅ OpenAI |
| `service_tier` | `Option<ServiceTier>` | ✅ OpenAI |
| `verbosity` | `Option<Verbosity>` | ✅ OpenAI |
| `prompt_cache_key` | `Option<String>` | ✅ OpenAI |
| `cache_control` | `Option<CacheControl>` | ✅ Anthropic (block-level) + OpenAI (request-level) |
| `extra_body` | `Option<Value>` | ✅ OpenAI 兼容层 (透传逃逸) |

### ContentPart — 内容块 ✅ genai

| 变体 | 说明 |
|---|---|
| `Text(String)` | 纯文本 |
| `Binary(Binary)` | 图片/PDF/音频 |
| `ToolCall(ToolCall)` | 工具调用 |
| `ToolResponse(ToolResponse)` | 工具结果 |
| `ThoughtSignature(String)` | 已签名的思考过程 |
| `ReasoningContent(String)` | 推理内容（与 ThoughtSignature 区分） |
| `Custom(CustomPart)` | 厂商特有内容（genai 未覆盖的内容块类型由此承载） |

### Usage — Token 用量 ✅ genai

| 字段 | 类型 |
|---|---|
| `prompt_tokens` | `Option<i32>` |
| `completion_tokens` | `Option<i32>` |
| `total_tokens` | `Option<i32>` |
| `prompt_tokens_details` | `Option<PromptTokensDetails>` |
| `completion_tokens_details` | `Option<CompletionTokensDetails>` |

---

## 适配器层

### Protocol — 入站协议枚举 🆕（精简）

```rust
pub enum Protocol {
    OpenAiChat,      // /v1/chat/completions
    OpenAiResponses, // /v1/responses
    Anthropic,       // /v1/messages
}
```

与 genai `AdapterKind` 的关系:

| Protocol (入站) | genai AdapterKind (出站) | 入站适配器 |
|---|---|---|
| `OpenAiChat` | 任意 | `adapters/openai` |
| `OpenAiResponses` | 任意 | `adapters/openai-resp` |
| `Anthropic` | 任意 | `adapters/anthropic` |

### Adapter trait 🆕（替代 Codec）

```rust
pub trait Adapter: Send + Sync {
    fn protocol(&self) -> Protocol;

    fn decode_request(&self, body: &[u8]) -> Result<ChatRequest, AdapterError>;
    fn encode_response(&self, response: &ChatResponse) -> Result<Vec<u8>, AdapterError>;
    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError>;
    fn encode_error(&self, error: &AdapterError) -> Vec<u8>;
}
```

### AdapterError 🆕

```rust
pub enum AdapterError {
    Decode(String),
    Encode(String),
    Unsupported(String),
}
```

---

## 配置层

### ProviderConfig 🆕

```yaml
providers:
  <name>:
    format: openai | anthropic | openai_resp | gemini | groq | ...
    url: "https://api.openai.com"        # 后端 API 基础 URL
    api_key: "${OPENAI_API_KEY}"         # API Key 或环境变量引用
    models: ["gpt-5.*", "gpt-4.1-*"]   # 路由匹配模型 pattern
```

### RouteRule 🔄（保留现有结构）

| 字段 | 类型 | 说明 |
|---|---|---|
| `models` | `Vec<String>` | 模型名称 pattern（支持 `*` 通配符） |
| `provider` | `String` | 目标 provider 名称 (map key) |
| `conditions` | `Option<RouteConditions>` | 附加匹配条件 |

路由匹配后的下游调用:

```
RouteRule.provider → ProviderConfig {
    format → genai AdapterKind,
    url → genai Endpoint,
    api_key → genai AuthData,
    models → 已通过路由匹配
}
→ ServiceTarget → genai Client
```

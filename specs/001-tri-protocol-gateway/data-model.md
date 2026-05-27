# 数据模型: LLM Mux 三协议互转网关

**Created**: 2026-05-27 | **Source**: [spec.md](./spec.md) + [crates/llm-mux-core/src/](../crates/llm-mux-core/src/)

> 以下模型均为已实现代码的文档化描述。标注 ✅ 表示已实现，❌ 表示待创建。

## 核心 IR 实体 (llm-mux-core)

### IrRequest — 统一请求 ✅

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `model` | `String` | 是 | 请求的目标模型标识 |
| `messages` | `Vec<IrMessage>` | 是 | 对话消息列表 |
| `system_prompt` | `Option<String>` | 否 | 系统提示词（跨协议统一映射） |
| `tools` | `Option<Vec<IrTool>>` | 否 | 可用工具定义 |
| `tool_choice` | `Option<IrToolChoice>` | 否 | 工具选择策略 |
| `max_tokens` | `Option<u32>` | 否 | 最大生成 Token 数 |
| `temperature` | `Option<f64>` | 否 | 采样温度 |
| `top_p` | `Option<f64>` | 否 | 核采样参数 |
| `top_k` | `Option<u32>` | 否 | Top-K 采样参数 |
| `stop_sequences` | `Option<Vec<String>>` | 否 | 停止序列 |
| `stream` | `bool` | 否 | 是否流式响应 |
| `thinking` | `Option<IrThinkingConfig>` | 否 | 思考/推理配置 |
| `response_format` | `Option<IrResponseFormat>` | 否 | 输出格式约束 |
| `provider_extensions` | `ProviderExtensions` | 否 | 厂商特有字段透传 (HashMap) |
| `inbound_protocol` | `Protocol` | (skip) | 入站协议，序列化时跳过 |
| `original_model` | `Option<String>` | (skip) | 原始模型名，序列化时跳过 |
| `raw_extra` | `Option<HashMap<String, Value>>` | (skip) | 原始额外字段，序列化时跳过 |
| `outbound_extra` | `Option<HashMap<String, Value>>` | (skip) | 出站额外字段，序列化时跳过 |

### IrResponse — 统一响应 ✅

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `id` | `Option<String>` | 否 | 响应唯一标识 |
| `model` | `Option<String>` | 否 | 生成模型标识 |
| `content` | `Vec<ContentBlock>` | 是 | 响应内容块序列 |
| `stop_reason` | `Option<StopReason>` | 否 | 停止原因 |
| `stop_sequence` | `Option<String>` | 否 | 触发的停止序列 |
| `usage` | `IrUsage` | 否 | Token 用量统计 |
| `provider_extensions` | `ProviderExtensions` | 否 | 厂商特有字段 |

### IrStreamEvent — 统一流事件 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `event_type` | `StreamEventType` | 事件类型 |
| `response` | `Option<IrResponse>` | 完整响应快照 (Start 事件) |
| `index` | `i32` | 内容块索引 |
| `delta` | `Option<ContentBlock>` | 增量内容 |
| `stop_reason` | `Option<StopReason>` | 停止原因 (Message Delta) |
| `usage` | `Option<IrUsage>` | 用量更新 |
| `error` | `Option<StreamError>` | 流错误信息 |

### StreamEventType — 流事件类型枚举 ✅

| 变体 | 含义 |
|------|------|
| `Start` | 流开始 |
| `Delta` | 内容增量 |
| `ContentBlockStart` | 新内容块开始 |
| `ContentBlockStop` | 内容块结束 |
| `Stop` | 流结束 |
| `Error` | 流错误 |

### IrMessage — 对话消息 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `role` | `Role` | 消息角色 |
| `content` | `Vec<ContentBlock>` | 消息内容块 |

### Role — 角色枚举 ✅

| 变体 | 描述 |
|------|------|
| `System` | 系统消息 |
| `User` | 用户消息 |
| `Assistant` | 助手消息 |
| `Tool` | 工具结果消息 |

### ContentBlock — 内容块（判别联合） ✅

核心字段 `content_type: ContentType` 指示有效负载类型，对应一个同名字段：

| ContentType | 对应字段 | 描述 |
|-------------|----------|------|
| `Text` | `text: Option<TextContent>` | 纯文本 |
| `Image` | `image: Option<ImageContent>` | 图片引用 |
| `ToolUse` | `tool_use: Option<ToolUseContent>` | 工具调用 |
| `ToolResult` | `tool_result: Option<ToolResultContent>` | 工具结果 |
| `ServerToolUse` | `server_tool_use: Option<ServerToolUseContent>` | 服务端工具调用 |
| `WebSearchToolResult` | `web_search_tool_result: Option<WebSearchToolResultContent>` | 服务端搜索结果 |
| `Document` | `document: Option<DocumentContent>` | 文档引用 |
| `Thinking` | `thinking: Option<ThinkingContent>` | 思考内容 |
| `RedactedThinking` | `redacted_thinking: Option<RedactedThinkingContent>` | 脱敏思考 |
| `Refusal` | `refusal: Option<RefusalContent>` | 拒绝响应 |

所有 ContentBlock 均含 `citations: Vec<Citation>` 字段用于引用标注。

### 内容子类型

| 结构体 | 关键字段 | 用途 |
|--------|----------|------|
| `TextContent` | `text: String` | 文本 |
| `ImageContent` | `data: Option<String>`, `url: Option<String>`, `media_type: Option<String>` | 图片（base64 或 URL） |
| `ToolUseContent` | `id: String`, `name: String`, `arguments: Option<Value>` | 工具调用 |
| `ToolResultContent` | `tool_use_id: String`, `content: Vec<ContentBlock>`, `is_error: Option<bool>` | 工具执行结果 |
| `ThinkingContent` | `thinking: String`, `signature: String` | 思考内容 + 签名 |
| `RedactedThinkingContent` | `data: String` | 脱敏思考占位 |
| `RefusalContent` | `refusal: String` | 拒绝理由 |
| `DocumentContent` | `data`, `url`, `media_type`, `title` | 文档数据 |
| `ServerToolUseContent` | `id`, `name`, `arguments` | 服务端工具（如 web_search） |
| `WebSearchToolResultContent` | `tool_use_id`, `content: Vec<WebSearchResult>`, `is_error`, `error_code` | 服务端搜索结果 |
| `WebSearchResult` | `title: String`, `url: String` | 搜索结果条目 |
| `Citation` | `kind`, `title`, `url`, `start`, `end`, `source_id` | 引用标注 |

### IrTool — 工具定义 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `type` | `String` | 工具类型 ("function") |
| `name` | `String` | 工具名称 |
| `description` | `Option<String>` | 工具描述 |
| `parameters` | `Value` | JSON Schema 参数定义 |
| `strict` | `Option<bool>` | 严格模式 |
| `extra_fields` | `HashMap<String, Value>` | 厂商特有字段 |

### IrToolChoice — 工具选择 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `choice_type` | `String` | "auto" / "any" / "none" / "tool" |
| `tool_name` | `Option<String>` | 指定工具名 (choice_type="tool" 时) |
| `allowed_tool_names` | `Option<Vec<String>>` | 允许的工具列表 |
| `allow_parallel_calls` | `Option<bool>` | 是否允许并行调用 |

### IrThinkingConfig — 思考配置 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `mode` | `Option<String>` | 思考模式 |
| `budget_tokens` | `Option<u32>` | Token 预算 |
| `effort` | `Option<String>` | 推理努力程度 |
| `include_thoughts` | `Option<bool>` | 是否包含思考 |
| `level` | `Option<String>` | 思考级别 |

### IrResponseFormat — 输出格式 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `format_type` | `String` | "text" / "json_schema" |
| `json_schema` | `Option<Value>` | JSON Schema 定义 |

### IrUsage — Token 用量 ✅

| 字段 | 类型 | 描述 |
|------|------|------|
| `input_tokens` | `Option<u32>` | 输入 Token 数 |
| `output_tokens` | `Option<u32>` | 输出 Token 数 |
| `total_tokens` | `Option<u32>` | 总计 Token 数 |
| `cache_read_tokens` | `Option<u32>` | 缓存读取 Token |
| `cache_creation_tokens` | `Option<u32>` | 缓存创建 Token |
| `thinking_tokens` | `Option<u32>` | 思考 Token |

### Protocol — 协议枚举 ✅

| 变体 | 描述 | Codec 状态 |
|------|------|------------|
| `OpenAiChat` | OpenAI Chat Completions API | ✅ 已实现 |
| `OpenAiResponses` | OpenAI Responses API | ❌ 待创建 |
| `Anthropic` | Anthropic Messages API | ✅ 已实现 |

### 编解码器 trait 体系 ✅

| Trait | 职责 | 实现状态 |
|-------|------|----------|
| `Codec` | 协议 ↔ IR 编解码 | ✅ OpenAI Chat, Anthropic Messages |
| `Router` | 请求路由决策 | ✅ FixedRouter (简单实现) |
| `Converter` | 跨协议字段适配 | ✅ NoopConverter (空实现) |
| `Authenticator` | API Key 验证 | ❌ 待实现 |

## 服务配置实体 (llm-mux-gateway) ❌ 待创建

### ServerConfig — 服务配置

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `port` | `u16` | 是 | HTTP 监听端口 |
| `host` | `String` | 是 | 绑定地址 |
| `log_level` | `LogLevel` | 否 | 日志级别 |
| `drain_timeout_secs` | `u64` | 否 | 优雅关闭超时 (default: 30) |
| `providers` | `HashMap<String, ProviderConfig>` | 是 | 上游 Provider 定义（按名称索引） |
| `routes` | `Vec<RouteConfig>` | 是 | 路由规则列表 |
| `api_keys` | `Vec<String>` | 否 | 允许的 API Key 列表 |

### ProviderConfig — 上游 Provider

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `protocol` | `Protocol` | 是 | 目标协议 |
| `base_url` | `String` | 是 | 目标 API 地址 |
| `api_key` | `String` | 是 | API Key（支持 `${ENV}` 展开） |
| `headers` | `HashMap<String, String>` | 否 | 额外 HTTP 请求头 |
| `model_mapping` | `HashMap<String, String>` | 否 | 入站模型名 → 后端模型名（支持 `*` 通配符兜底） |

### RouteConfig — 路由规则

- **匹配策略**: 从上到下首个命中生效（first-match-wins）
- **兜底规则**: 最后一条必须为 `models: ["*"]`，不含其他条件
- **匹配语义**: 所有条件 AND 关系，全部满足才命中

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `models` | `Vec<String>` | 是 | 模型名模式（支持 `*` / `?` 通配符） |
| `protocol` | `Option<Protocol>` | 否 | 匹配入站协议 |
| `stream` | `Option<bool>` | 否 | 匹配流式/非流式请求 |
| `has_tools` | `Option<bool>` | 否 | 匹配是否含工具定义 |
| `has_media` | `Option<bool>` | 否 | 匹配是否含媒体内容 |
| `provider` | `String` | 是 | 命中后路由到的 Provider 名称 |

### LogLevel — 日志级别

`error | warn | info | debug | trace`，默认 `info`。

## 数据流生命周期

```
Client Request (Protocol A bytes)
  → CodecA.decode_request() → IrRequest
    → Authenticator.authenticate() → OK
      → Router.route() → RouteResult (Protocol B)
        → Converter.convert_request(IrRequest, RouteResult)
          → CodecB.encode_request() → Protocol B bytes
            → HTTP POST to backend
              ← Backend Response (Protocol B bytes / SSE stream)
                → [if stream: CodecB.decode_stream_event() per event]
                → [if non-stream: construct IrResponse from response JSON]
                  → Converter.convert_response(IrResponse)
                    → CodecA.encode_response() → Protocol A bytes / SSE events
                      → Client receives response
```

每一步均在 `tracing` span 内执行，span 携带 `request_id` 和 `model` 字段。

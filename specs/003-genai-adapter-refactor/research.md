# 技术调研: 基于 genai 适配器模式重构协议网关

**日期**: 2026-05-28 | **来源**: [plan.md](./plan.md) Phase 0 | [genai 兼容性报告](../../docs/rust-genai-compatibility-report.md)

---

## 1. genai IR 类型映射到现有自定义 IR

### 决策

复用 genai 的类型体系作为统一 IR，删除现有的自定义 IR 类型。

### 理由

genai 的类型体系（`ChatRequest`、`ChatResponse`、`ChatStreamEvent`、`MessageContent`、`ContentPart`、`Tool`、`ToolCall`、`ToolResponse`、`ChatOptions`、`Usage`）已经覆盖了 LLM Mux 自定义 IR 的 90%+ 语义。之前在 [rust-genai 兼容性报告](../../docs/rust-genai-compatibility-report.md) 中已验证 genai 对三种 API 的核心功能覆盖情况。

### 映射关系

| 现有自定义 IR 类型 | genai 对应类型 | 说明 |
|---|---|---|
| `IrRequest` | `ChatRequest` | genai 的 `messages` 字段替代 `IrRequest.messages` |
| `IrResponse` | `ChatResponse` | genai 的 `content`(MessageContent) 替代 `IrResponse.content` |
| `IrStreamEvent` | `ChatStreamEvent` | genai 流事件枚举（`Start`/`Chunk`/`ReasoningChunk`/`ToolCallChunk`/`End`）直接映射 |
| `IrMessage` | `ChatMessage` | genai 的 `role`+`content` 替代 |
| `ContentBlock` (enum) | `ContentPart` (enum) | genai 的 `Text`/`Binary`/`ToolCall`/`ToolResponse`/`ReasoningContent`/`Custom` 变体覆盖现有所有变体 |
| `Role` | `ChatRole` | System/User/Assistant/Tool 直接对应 |
| `StopReason` | `StopReason` | genai 已提供 provider-agnostic 标准化枚举 |
| `IrUsage` | `Usage` | genai 已提供 prompt_tokens/completion_tokens/total_tokens + details |
| `ProviderExtensions` | `ChatOptions.extra_body` + `ContentPart::Custom` | 透传机制 |

### 不可覆盖的特殊内容类型

| 内容类型 | 处理方式 |
|---|---|
| Anthropic `redacted_thinking` | `ContentPart::Custom(serde_json::Value)` — 保留原始 payload |
| Anthropic `server_tool_use` | `ContentPart::Custom` — genai 不支持内置工具调用传播 |
| Anthropic citations | `ContentPart::Custom` — genai 不捕获引用数据 |
| OpenAI Responses 内置工具调用 | `ContentPart::Custom` — genai 支持但未完整暴露 |

### 备选方案评估

- **方案 B: 保留自定义 IR，在适配器内做双层转换** — 增加概念层和内存分配，违背 FR-010 代码精简目标。放弃。
- **方案 C: 完全放弃 IR 层，每个协议直连** — 失去跨协议互转能力。放弃。

---

## 2. genai Client 集成方式

### 决策

LLM Mux 维护一个全局 `genai::Client` 实例，每次下游请求构造 `ModelSpec::Target(ServiceTarget)` 精确指定目标端点、认证和模型。

### 理由

genai Client 内部管理 `reqwest::Client` 连接池，多 provider 之间共享一个 `reqwest::Client` 更高效。通过 `ModelSpec::Target(ServiceTarget)` 绕过 genai 的模型名 → AdapterKind 推断逻辑，直接控制目标协议。

### ServiceTarget 构造流程

```
Provider Config (config.yaml)
  ├── format: "anthropic"        → genai AdapterKind::Anthropic
  ├── url: "https://api.anthropic.com"
  ├── api_key: "${ANTHROPIC_API_KEY}"
  └── models: ["claude-sonnet-4-5"]
          ↓
Route Match (model_name → provider)
          ↓
ServiceTarget {
    model: ModelIden { adapter_kind: Anthropic, model_name: "claude-sonnet-4-5" },
    endpoint: Endpoint::from_owned("https://api.anthropic.com"),
    auth: AuthData::Key("<resolved-api-key>"),
}
          ↓
genai Client: client.exec_chat(ModelSpec::Target(service_target), chat_request, &chat_options)
```

### 备选方案评估

- **方案 B: 每 provider 创建独立 Client** — 连接池分散，管理复杂。放弃。
- **方案 C: 使用 genai 的 `ClientBuilder::with_service_target_resolver()`** — 适合静态路由，但 LLM Mux 需要动态路由（每个请求可能匹配不同 provider），不适合。放弃。

---

## 3. genai Client 连接池配置

### 决策

genai Client 内部使用 `reqwest::Client` 的默认连接池，不额外配置。如需调整连接池大小，通过 genai 的 `WebConfig` 设置。

### 理由

genai 默认的 `reqwest::Client` 已包含合理的连接池配置（默认最大空闲连接 10、keepalive 90s）。LLM Mux 原有 `FR-021` 连接池参数（connect timeout 5s / read timeout 300s / pool 128）可通过 genai ClientBuilder 的 `with_web_config()` 方法透传。

### genai WebConfig 映射

| LLM Mux 参数 | genai 配置方式 |
|---|---|
| connect timeout | `WebConfig::with_connect_timeout(Duration)` |
| read timeout | `WebConfig::with_read_timeout(Duration)` |
| pool size | genai 内部 reqwest 默认，不暴露显式设置 |
| keepalive | genai 内部 reqwest 默认 90s |

---

## 4. Adapter trait 设计

### 决策

定义 `Adapter` trait 替代现有 `Codec` trait，职责缩小为仅"入站解码 + 出站编码"，不再包含 HTTP 调用和请求构造逻辑。

### 理由

现有 `Codec` trait 同时包含 `encode_request()`（构造下游 HTTP 请求体）和 `decode_response()`（解析下游 HTTP 响应），这些职责在引入 genai 后不再需要——genai 自动处理。新的 `Adapter` trait 专注于外部协议 ↔ IR 的翻译。

### 方法对比

| 现有 Codec trait 方法 | 新 Adapter trait 方法 | 变化 |
|---|---|---|
| `decode_request(bytes) → IrRequest` | `decode_request(bytes) → ChatRequest` | 返回类型改为 genai 类型 |
| `decode_response(bytes) → IrResponse` | **删除** — genai 自动处理 | — |
| `encode_response(response) → bytes` | `encode_response(response) → bytes` | 输入类型改为 genai `ChatResponse` |
| `encode_request(request) → bytes` | **删除** — genai 自动处理 | — |
| `decode_stream_event(...) → IrStreamEvent` | **删除** — genai 自动处理下游流，入站不需要 | — |
| `encode_stream_event(event) → String` | `encode_stream_event(event) → String` | 输入改为 genai `ChatStreamEvent` |
| `known_fields()` | **删除** — extra_body 覆盖此功能 | — |

### 备选方案评估

- **方案 B: 保留 Codec trait，内部委托 genai** — 保留不必要的双层抽象。放弃。

---

## 5. 配置格式变更

### 决策

Provider 配置从:

```yaml
# 旧格式
providers:
  - name: my-claude
    protocol: anthropic  # ← 改为 format
    endpoint: https://api.anthropic.com  # ← 改为 url
```

改为:

```yaml
# 新格式
providers:
  my-claude:
    format: anthropic   # genai AdapterKind 可序列化名称
    url: https://api.anthropic.com
    api_key: ${ANTHROPIC_API_KEY}
    models: ["claude-sonnet-4-5", "claude-haiku-4-5"]
```

### 理由

- `format` 替代 `protocol`：直接对应 genai `AdapterKind`，避免二次映射
- `url` 替代 `endpoint`：更简洁，且与 genai `Endpoint` 概念对齐
- Map 格式替代数组格式：YAML 自然表达，provider 名称作为 key 使路由引用更清晰

### 字段映射

| 当前字段 | 新字段 | 说明 |
|---|---|---|
| `name`（数组中的字段） | map key | Provider 唯一标识 |
| `protocol` | `format` | genai AdapterKind（openai/anthropic/openai_resp/gemini/...） |
| `endpoint` | `url` | 后端 API 基础 URL |
| `api_key` | `api_key` | 不变 |
| `models` | `models` | 不变（路由匹配 pattern 列表） |
| `api_version` | 移除 | genai 自行管理 API 版本 |
| `auth_header` | 移除 | genai `AuthData` 自动处理 |

---

## 6. 错误映射: genai::Error → 协议格式

### 决策

定义 `ErrorMapper` 工具函数，将 `genai::Error` 映射为客户端协议格式的 JSON 错误响应体。

### 映射规则

```
genai::Error → HTTP Status → Client Protocol Error Body
──────────────────────────────────────────────────────
WebModelCall / WebAdapterCall → 502 Bad Gateway →  {error: {type: "server_error", message: "..."}}
HttpError { status: 429 }     → 429 Too Many     →  {error: {type: "rate_limit_exceeded", message: "..."}}
HttpError { status: 401/403 } → 502 Bad Gateway   →  {error: {type: "server_error", message: "..."}} (后端密钥问题)
ChatResponseGeneration        → 502 Bad Gateway   →  {error: {type: "server_error", message: "..."}}
StreamParse                   → 502 Bad Gateway   →  {error: {type: "server_error", message: "..."}}
NoChatResponse                → 502 Bad Gateway   →  {error: {type: "server_error", message: "..."}}
RequiresApiKey / NoAuthData   → 500 Internal      →  {error: {type: "server_error", message: "..."}} (配置问题)
Serializer / Deserializer     → 400 Bad Request   →  {error: {type: "invalid_request_error", message: "..."}}
```

错误 body 格式随入站协议有所不同:
- OpenAI Chat: `{"error": {"type": "...", "message": "...", "code": "..."}}`
- Anthropic Messages: `{"type": "error", "error": {"type": "...", "message": "..."}}`
- OpenAI Responses: `{"type": "error", "error": {"type": "...", "message": "..."}}`

### 备选方案评估

- **方案 B: 在 genai Error 上实现 Into<协议错误>** — 每个适配器需要独立实现，增加样板代码。放弃。采用中心化映射 + 按协议格式化。

---

## 7. 迁移策略

### 决策

分阶段迁移，保持网关在迁移期间可运行。

### 理由

一次性全量替换风险高，特别是集成测试覆盖不完全时。

### 迁移阶段

1. **阶段 1: 引入 genai 依赖，保持现有架构不变**
   - 在 workspace `Cargo.toml` 中添加 genai 依赖
   - 验证 genai 可编译、类型可用

2. **阶段 2: 用 genai 类型替换 IR**
   - 在 `core` 中删除 `ir.rs`，改为从 genai 重导出
   - 更新所有现有的 `use crate::ir::*` 引用
   - 逐个适配器更新 `decode_request` / `encode_response` 方法签名

3. **阶段 3: 用 genai Client 替换 HTTP 客户端**
   - 在 `gateway` 中删除 reqwest 依赖
   - 实现 `ServiceTarget` 构造器
   - 替换下游调用路径中的 HTTP 请求为 genai Client 调用

4. **阶段 4: 清理与优化**
   - 删除不再使用的 `known_fields`、`encode_request`、`decode_response` 方法
   - 删除 `Converter` trait（如被 genai 透传机制取代）
   - 更新配置格式（`protocol` → `format`）
   - 运行全套测试验证

5. **阶段 5: 配置格式切换**
   - 支持新 old config warning → 新 config 格式
   - 废弃旧字段名

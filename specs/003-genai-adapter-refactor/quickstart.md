# 快速开始: 基于 genai 适配器模式重构

**日期**: 2026-05-28

---

## 迁移概览

本指南描述从当前自建 IR + reqwest 架构迁移到 genai 适配器架构的步骤。

### 变更摘要

| 组件 | 变更前 | 变更后 |
|---|---|---|
| IR 类型 | 自定义 `IrRequest`/`IrResponse`/`IrStreamEvent` | genai `ChatRequest`/`ChatResponse`/`ChatStreamEvent` |
| Content 类型 | 自定义 `ContentBlock` | genai `ContentPart` |
| HTTP 客户端 | 自建 reqwest + SSE 手动解析 | genai `Client`（内置 HTTP + SSE） |
| 编解码 trait | `Codec`（双向 + HTTP 构造） | `Adapter`（仅入站翻译） |
| 配置格式 | 数组 `protocol: anthropic` | Map `format: anthropic` |

---

## 步骤 1: 添加 genai 依赖

在 workspace `Cargo.toml` 的 `[workspace.dependencies]` 中添加:

```toml
genai = "0.6"
```

在 `crates/core/Cargo.toml` 中添加:

```toml
[dependencies]
genai.workspace = true
```

在 `crates/gateway/Cargo.toml` 中:

```toml
# 替换
reqwest = { workspace = true, features = ["stream", "json"] }
# 为
genai.workspace = true
```

---

## 步骤 2: 替换 IR 类型

### 2.1 删除 crates/core/src/ir.rs

### 2.2 更新 crates/core/src/lib.rs

```rust
// 自建 IR（删除）
// pub mod ir;
// pub use ir::*;

// genai IR（新增）
pub use genai::chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamEvent,
    ChatOptions, ChatResponseFormat, ChatRole, ChatStream,
};
pub use genai::chat::{
    ContentPart, MessageContent, Tool, ToolCall, ToolResponse,
    StopReason, Usage, ReasoningEffort, ToolChoice,
};
```

### 2.3 更新适配器代码的 import

将适配器 crate 中的:

```rust
use llm_mux_core::ir::{IrRequest, IrResponse, IrStreamEvent, ContentBlock};
use llm_mux_core::codec::Codec;
```

改为:

```rust
use llm_mux_core::adapter::Adapter;
use llm_mux_core::{ChatRequest, ChatResponse, ChatStreamEvent, ContentPart};
```

---

## 步骤 3: 实现新 Adapter trait

### 3.1 以 openai-chat 适配器为例

```rust
pub struct OpenAiChatAdapter;

impl Adapter for OpenAiChatAdapter {
    fn protocol(&self) -> Protocol { Protocol::OpenAiChat }

    fn decode_request(&self, body: &[u8]) -> Result<ChatRequest, AdapterError> {
        let req: CreateChatCompletionRequest = serde_json::from_slice(body)?;
        // 转换为 ChatRequest ...
    }

    fn encode_response(&self, response: &ChatResponse) -> Result<Vec<u8>, AdapterError> {
        // ChatResponse → OpenAI 响应 JSON
    }

    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError> {
        // ChatStreamEvent → SSE data line
    }

    fn encode_error(&self, error: &AdapterError) -> Vec<u8> { /* ... */ }
}
```

### 3.2 删除不再需要的代码

每个适配器中删除:
- `encode_request()` 方法及其实现
- `decode_response()` 方法及其实现
- `decode_stream_event()` 方法及其实现
- `known_fields()` 方法及其实现
- SSE 解析器（由 genai streamer 替代）

---

## 步骤 4: 替换 gateway HTTP 客户端

### 4.1 删除 reqwest 依赖

移除 `crates/gateway/Cargo.toml` 中的 `reqwest`、`futures`、`tokio-stream`、`bytes` 等直接 HTTP 相关依赖。

### 4.2 添加 genai Client

```rust
use genai::Client;

// 全局 Client（AppState）
let client = Client::default();

// 下游调用
let service_target = ServiceTarget {
    model: ModelIden::new(adapter_kind, model_name.clone()),
    endpoint: Endpoint::from_owned(provider.url.clone()),
    auth: AuthData::from_single(provider.api_key.clone()),
};

let chat_res = client.exec_chat(
    ModelSpec::Target(service_target),
    chat_request,
    Some(&chat_options),
).await?;
```

### 4.3 错误映射

```rust
fn map_genai_error(error: genai::Error, protocol: Protocol) -> (StatusCode, Vec<u8>) {
    match error {
        genai::Error::HttpError { status, body, .. } => {
            let status_code = StatusCode::from_u16(status as u16).unwrap_or(StatusCode::BAD_GATEWAY);
            let error_body = protocol_error_body(protocol, status_code, &body);
            (status_code, error_body)
        }
        _ => (StatusCode::BAD_GATEWAY, protocol_error_body(protocol, StatusCode::BAD_GATEWAY, &error.to_string())),
    }
}
```

---

## 步骤 5: 更新配置格式

### 旧格式（删除）

```yaml
providers:
  - name: claude
    protocol: anthropic
    endpoint: https://api.anthropic.com
    api_key: key
    models: ["claude-sonnet-4-5"]
```

### 新格式

```yaml
providers:
  claude:
    format: anthropic
    url: https://api.anthropic.com
    api_key: ${ANTHROPIC_API_KEY}
    models: ["claude-sonnet-4-5"]
```

配置结构体变更:

```rust
// 旧
struct ProviderConfig {
    name: String,
    protocol: String,
    endpoint: String,
    api_key: String,
    models: Vec<String>,
}

// 新
struct ProviderConfig {
    format: String,           // genai AdapterKind
    url: String,              // API base URL
    api_key: String,          // 支持 ${ENV_VAR} 引用
    models: Vec<String>,      // 路由匹配 pattern
}
```

解析 `api_key` 中的 `${ENV_VAR}` 引用，按环境变量展开。

---

## 步骤 6: 运行测试

```bash
cargo test --workspace           # 单元测试
cargo test --test cross_protocol # 集成测试
cargo clippy --workspace         # lint
cargo fmt --check --all          # 格式
```

---

## 常见问题

**Q: 为什么不能同时使用旧 Codec 和新 Adapter trait？**

旧 Codec trait 的方法签名与 genai 类型耦合后无法直接共存。分阶段迁移期间，可先让适配器同时实现两个 trait（桥接模式），待全部适配器迁移后删除 Codec。

**Q: genai 不支持我的后端特殊参数怎么办？**

通过 `ChatOptions.extra_body` 透传。genai 的 OpenAI-compatible adapter 会将 extra_body 合并入请求体。对于 Anthropic adapter，extra_body 不被支持（这是 genai 的限制），此时记录 WARN 日志。

**Q: 现有集成测试如何适应新架构？**

集成测试的核心逻辑（构造请求、发送、验证响应字段）保持不变。只需更新测试中的 import（`IrRequest` → `ChatRequest`）和适配器初始化（`Codec` → `Adapter`）。

# 架构文档 — Gateway

## 概要

LLM Mux 是一个高性能 LLM API 协议互转网关。通过统一的 Internal Representation（IR）层，将 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种入站协议双向互转到多种出站后端。

## 技术栈

| 类别 | 技术 |
|------|------|
| 语言 | Rust 2021 edition |
| Web 框架 | axum 0.7 |
| LLM SDK | genai v0.6（外部 crate） |
| 运行时 | tokio 1（多线程） |
| 序列化 | serde 1 + serde_json 1 |
| 配置 | serde_yaml 0.9 + dotenvy |
| 日志 | tracing 0.1 + tracing-subscriber 0.3 |
| CLI | clap 4 |

## 架构模式

**适配器模式** — 每种入站协议实现 `Codec` trait，负责：
1. `decode_request`: 外部协议 JSON → 内部 IR
2. `encode_response`: 内部 IR → 外部协议 JSON
3. `encode_error`: 错误 → 外部协议错误格式

### 请求流程（非流式）

```
Client → HTTP POST
  → 认证中间件（API Key 白名单）
    → 入站 Codec.decode_request() → IrRequest
      → Router.route() → 选择目标 Provider
        → 构造 genai ChatRequest
          → genai Client.exec_chat()
            → genai ChatResponse
          ←
        → ir_from_genai_response() → IrResponse
      ←
    → 入站 Codec.encode_response() → JSON
  ←
HTTP Response → Client
```

### 请求流程（流式）

```
Client → HTTP POST (stream: true)
  → 认证中间件
    → 入站 Codec.decode_request() → IrRequest
      → Router.route()
        → 构造 genai ChatRequest
          → genai Client.exec_chat_stream()
            → ChatStream（futures::Stream）
          ←
        → genai_stream_to_sse()
          ← SSE events
      ←
    ← SSE stream
  ←
HTTP Response (SSE) → Client
```

### 路由系统

- 配置驱动，自上而下首个匹配生效
- 支持多条件匹配：模型名（glob）、入站协议、是否流式、是否有工具、是否有媒体
- 支持模型名映射（如 `gpt-4o` → `claude-sonnet-4-6`）
- 兜底规则必须无条件且放在最后

### 出站调用

出站使用 genai crate 的 `AdapterKind` 命名空间语法：
- `openai::gpt-4o`
- `anthropic::claude-sonnet-4-6`
- `opencode_go::gpt-5`

## 项目结构

详见 [source-tree-analysis.md](./source-tree-analysis.md)

## 测试策略

- **单元测试**：每个 adapter crate 独立测试 decode/encode 逻辑
- **集成测试**：`gateway/tests/` 中的跨协议 roundtrip、流式、e2e 测试
- 测试数据使用内嵌 JSON 字符串，无外部 fixture

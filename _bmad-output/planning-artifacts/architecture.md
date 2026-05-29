---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
lastStep: 8
status: 'complete'
completedAt: '2026-05-29'
inputDocuments:
  - "_bmad-output/planning-artifacts/prds/prd-llm-mux-2026-05-29/prd.md"
  - "_bmad-output/project-context.md"
  - "docs/architecture/architecture-gateway.md"
  - "docs/index.md"
workflowType: 'architecture'
project_name: 'llm-mux'
user_name: 'W'
date: '2026-05-29'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:** 19 FRs across 6 feature groups:
- **三协议入站解析** (FR-1~4) — OpenAI Chat / Anthropic Messages / OpenAI Responses 请求解析
- **跨协议响应编码** (FR-5~8) — 三种协议的响应/流事件/错误编码
- **可配置路由** (FR-9~10) — 多条件 glob 匹配 + 模型名映射
- **流式传输** (FR-11~12) — 逐事件 SSE 翻译，零缓冲
- **认证与安全** (FR-13) — API Key 白名单
- **运维与部署** (FR-14~19) — CLI / Docker / 健康检查 / 优雅关闭 / 请求 ID / 结构化日志

**Non-Functional Requirements:** 12 NFRs in 5 categories:
- Performance: 请求 ≤ 1ms, 流事件 ≤ 100μs, 100 并发 p95 ≤ 2ms, 内存 ≤ 50MB
- Security: unsafe forbid, API Key 日志脱敏
- Reliability: 后端异常不崩溃, 优雅关闭
- Observability: 结构化日志, 慢请求 WARN
- Compatibility: 跨平台, 未知字段透传

**Scale & Complexity:**
- Complexity level: Medium
- Primary domain: Backend API Gateway
- Components: 5 crates (core, gateway, 3 adapters)

### Technical Constraints & Dependencies

- **Language/Version:** Rust 2021 edition, workspace resolver = "2"
- **Core dependency:** genai v0.6 (LLM protocol crate)
- **Web framework:** axum 0.7
- **Async runtime:** tokio 1 (full features)
- **Build constraints:** LTO, codegen-units=1, opt-level="z", strip, panic="abort"
- **Code quality:** unsafe_code = "forbid", missing_docs = "warn", clippy pedantic + nursery
- **File limits:** ≤ 400 lines/file, ≤ 80 lines/method

### Cross-Cutting Concerns Identified

1. **协议保真度** — 三种协议的差异化和共同字段映射，extra_body 透传
2. **流式事件翻译正确性** — SSE 事件序列、类型映射、终止信号
3. **工具调用完整性** — tool_use ↔ tool_calls 双向无损
4. **内容类型覆盖** — Text / Image / ToolUse / ToolResult / Thinking / Refusal 等 ContentBlock
5. **错误传播** — 后端错误 → 入站协议错误格式映射

## Core Architectural Decisions

### Data Architecture

| 决策 | 选择 | 说明 |
|------|------|------|
| 数据模型 | 无状态 | 网关不持久化任何数据，所有状态在请求生命周期内 |
| IR 类型系统 | 自建（IrRequest/IrResponse/IrStreamEvent） | 自定义中间层，与 genai 类型解耦 |
| 内容模型 | ContentBlock 判别联合 | text/image/tool_use/tool_result/thinking/refusal 等 |
| 适配器主接口 | Codec trait（`decode_request → IrRequest`） | 适配器不依赖 genai |
| 适配器旧接口 | Adapter trait（使用 genai 类型） | 遗留桥接层，非首选 |
| 协议特有字段 | ProviderExtensions + raw_extra | 自定义 IR 层独立承载，不依赖 genai extra_body |

### Authentication & Security

| 决策 | 选择 |
|------|------|
| 认证方式 | API Key 白名单（可选，空列表=不验证） |
| 安全约束 | `unsafe_code = "forbid"` 全局禁止 unsafe |
| 密钥管理 | 配置文件 + 环境变量（`${VAR}`），日志脱敏 |

### API & Communication Patterns

| 决策 | 选择 |
|------|------|
| 接口风格 | REST（3 个 POST 端点 + 1 个 GET 健康检查） |
| 协议识别 | HTTP 路径自动识别，无需配置 |
| 出站调用 | genai Client + ServiceTarget |
| 路由引擎 | 多条件 AND 匹配，通配符 glob，自上而下命中 |
| 错误处理 | 后端错误 → 入站协议格式错误体 |

### Streaming Architecture

| 决策 | 选择 |
|------|------|
| 流式模式 | 逐事件翻译（genai ChatStream → SSE） |
| 缓冲策略 | 零缓冲，使用 futures::StreamExt::next() |
| 错误处理 | 流中断 → SSE error 事件 + 连接关闭 |
| Anthropic SSE | 自动注入 message_start + content_block_start 事件 |

### Infrastructure & Deployment

| 决策 | 选择 |
|------|------|
| 交付形态 | 单一静态二进制 + Docker 镜像 |
| 构建策略 | 多阶段 Docker（rust:1.85-slim → distroless/cc） |
| 运行用户 | nobody（65534:65534） |
| 优雅关闭 | SIGTERM/Ctrl+C + drain timeout（默认 30s） |
| CLI | clap derive，5 个子命令 |
| 跨平台 | cfg(unix) gate Unix-only 代码（libc） |

## Architecture Validation Results

### Coherence Validation ✅

- **决策兼容性：** 所有技术选型已在实际代码中验证兼容
- **模式一致性：** 自定义 IR + Codec trait 的模式已贯穿所有 adapter 实现
- **结构对齐：** crate 结构与架构分层一致（core:IR+Codec → adapters:编解码 → gateway:HTTP+genai）

### Requirements Coverage Validation ✅

- **FR 覆盖：** 19 个 FR 全部映射到具体 crate 和文件
- **NFR 覆盖：** 12 个 NFR 已分类记录
- **跨切面关注点：** 协议保真度、工具调用、流式翻译均已覆盖

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**Architectural Decisions**
- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**Implementation Patterns**
- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**Project Structure**
- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

**Overall Status：** READY FOR IMPLEMENTATION

## Implementation Patterns & Consistency Rules

### Naming Patterns

| 范围 | 规则 |
|------|------|
| 函数/变量 | snake_case |
| 类型/Trait/Enum | PascalCase |
| Enum 变体 | PascalCase |
| Serde rename | `rename_all = "snake_case"` 或 `"lowercase"` |
| 错误类型 | `{Module}Error`（如 `CodecError`, `AdapterError`） |
| Crate 命名 | `{vendor}-{protocol}-codec`（如 `openai-chat-codec`） |

### Structure Patterns

- **模块组织：** 文件夹模块形式（`module_name/mod.rs` + 拆分文件）
- **单文件限制：** ≤ 400 行
- **单方法限制：** ≤ 80 行
- **Crate 布局：** `src/lib.rs`（导出）+ `src/{module}/mod.rs`（子模块）+ `tests/`（集成测试）
- **Trait 定义：** 核心 trait 在 `core` crate 定义，实现放在各 adapter crate

### Format Patterns

- **JSON 字段命名：** snake_case（serde 默认）
- **Serde 可选字段：** `#[serde(skip_serializing_if = "Option::is_none")]`
- **Serde 集合：** `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- **Error 类型：** `thiserror` derive + `#[error("...")]` 格式化

### IR 中间层架构

```
入站 HTTP → Codec.decode_request() → IrRequest
                                         ↓
                                gateway 层：ir → genai 映射
                                         ↓
                                genai Client.exec_chat()
                                         ↓
                                gateway 层：genai → ir 映射
                                         ↓
                        IrResponse → Codec.encode_response() → 出站 HTTP
```

- **自建 IR 类型：** `IrRequest` / `IrResponse` / `IrStreamEvent` / `ContentBlock`（在 `core/src/ir.rs` + `types.rs`）
- **Codec trait（主接口）：** 使用自定义 IR 类型——`decode_request(body) → IrRequest`，`encode_response(IrResponse) → Vec<u8>`。适配器实现此 trait，**不依赖 genai**
- **Adapter trait（旧层）：** 使用 genai 类型，是遗留的桥接层，非首选接口
- **genai 定位：** 仅在 `gateway/src/handlers/` 中使用——将自定义 IR 映射到 genai 类型后调用 `exec_chat()`/`exec_chat_stream()`，再将 genai 响应映射回自定义 IR
- **切换灵活性：** 切换 AI Client 只需修改 gateway 层的 IR→Client 映射逻辑，所有 adapter 层不需要动
- **genai 重导出：** `core/src/lib.rs` 中对 genai 类型的重导出（`pub use genai::chat::{...}`）仅用于 gateway 层的便利，adapter 不应使用

### Enforcement Guidelines

| 规则 | 强制方式 |
|------|----------|
| unsafe_code | `[workspace.lints.rust]` 级别的 `forbid` |
| missing_docs | `[workspace.lints.rust]` 级别的 `warn` |
| clippy | pedantic + nursery + cargo 均为 `warn` |
| 行数限制 | 代码审查 |

## Project Structure & Boundaries

### Complete Project Directory Structure

```
llm-mux/
├── Cargo.toml                       # Workspace 定义（5 crates）
├── Cargo.lock
├── config.example.yaml              # 配置示例
├── Dockerfile                       # 多阶段构建
├── README.md
│
├── crates/
│   ├── core/                        # ── IR 类型 + Trait 定义 + 路由 + 认证
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # 公有 API 重导出
│   │       ├── ir.rs                # IrRequest / IrResponse / IrStreamEvent
│   │       ├── types.rs             # Protocol / ContentBlock / Role / StopReason
│   │       ├── adapter.rs           # Adapter trait（旧层，使用 genai 类型）
│   │       └── codec.rs             # Codec trait + Router + Authenticator + Converter
│   │
│   ├── gateway/                     # ── HTTP 服务 + CLI
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs              # CLI 入口（clap）
│   │       ├── lib.rs               # 库入口 + tracing 初始化
│   │       ├── config.rs            # YAML 配置解析 + 校验
│   │       ├── server.rs            # axum Router 挂载 + 优雅关闭
│   │       ├── middleware.rs        # 请求 ID 中间件
│   │       └── handlers/
│   │           ├── mod.rs           # 路由处理 + genai 调用 + SSE 代理
│   │           └── genai_bridge.rs  # IrResponse ↔ genai ChatResponse 转换
│   │
│   └── adapters/                    # ── 入站协议适配器（纯数据转换，不依赖 genai）
│       ├── openai/                  # OpenAI Chat Completions
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs
│       │       ├── models.rs        # Chat Completion 请求/响应模型
│       │       └── chat/
│       │           ├── mod.rs
│       │           ├── adapter.rs   # Codec + Adapter 实现
│       │           ├── convert.rs   # 协议模型 ↔ IR 转换
│       │           └── encode.rs    # 流式事件编码
│       │
│       ├── anthropic/               # Anthropic Messages
│       │   └── ...（同 openai 结构）
│       │
│       └── openai-resp/             # OpenAI Responses
│           └── ...（同 openai 结构）
│
├── docs/                            # 项目知识库
│   ├── index.md
│   ├── overview/
│   ├── architecture/
│   ├── api/
│   ├── development/
│   ├── operations/
│   ├── reference/
│   └── research/
│
├── _bmad-output/                    # BMad 工作流产出
│   ├── project-context.md
│   └── planning-artifacts/
│
└── specs/                           # （即将移除）
```

### Requirements to Structure Mapping

| FR | Crate | 关键文件 |
|----|-------|---------|
| FR-1~3 入站解码 | `adapters/{vendor}/` | `src/{protocol}/adapter.rs` - `decode_request()` |
| FR-5~7 响应编码 | `adapters/{vendor}/` | `src/{protocol}/adapter.rs` - `encode_response()`/`encode_stream_event()` |
| FR-8 错误编码 | `adapters/{vendor}/` | `src/{protocol}/adapter.rs` - `encode_error()` |
| FR-4 协议识别 | `gateway/` | `src/handlers/mod.rs` - 端点路由 |
| FR-9~10 路由 | `core/` | `src/codec.rs` - ConfigurableRouter, RouteRule |
| FR-11~12 流式 | `gateway/` | `src/handlers/mod.rs` - genai_stream_to_sse() |
| FR-13 认证 | `core/` | `src/codec.rs` - ConfigAuthenticator |
| FR-14 CLI | `gateway/` | `src/main.rs` - clap Commands |
| FR-15~17 部署 | `gateway/` | `src/server.rs`, `Dockerfile`, `src/config.rs` |
| FR-18~19 可观测 | `gateway/` | `src/middleware.rs`, `src/lib.rs` |


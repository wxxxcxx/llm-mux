# Implementation Plan: LLM Mux 三协议互转网关

**Branch**: `001-tri-protocol-gateway` | **Date**: 2026-05-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-tri-protocol-gateway/spec.md`

## Summary

基于已实现的 2N Codec 架构（OpenAI Chat ↔ Anthropic Messages 双协议互转）和统一 IR 层，补充
OpenAI Responses 编解码器、构建 axum HTTP 服务层、CLI 配置接口和 Docker 部署支持，交付完整的
三协议互转网关。

## Technical Context

**Language/Version**: Rust Edition 2021 (stable, 1.70+)

**Primary Dependencies**:
- `serde` + `serde_json` — 协议解析（✅ 已集成）
- `thiserror` — 错误处理（✅ 已集成）
- `tracing` + `tracing-subscriber` — 结构化日志（✅ workspace dep 已声明，未使用）
- `axum` + `tokio` — HTTP 服务（❌ 待集成）
- `clap` — CLI 参数解析（❌ 待集成）
- `serde_yaml` — 配置文件解析（❌ 待集成）

**Storage**: N/A（无状态网关，不持久化数据）

**Testing**: `cargo test`（集成测试 + 单元测试）

**Target Platform**:
- Linux x86_64-aarch64 (Release binary < 15MB stripped)
- macOS x86_64-aarch64
- Docker (distroless/scratch 基础镜像)

**Project Type**: library + CLI + web-service

**Performance Goals**:
- 请求协议转换 ≤ 1ms（典型 < 10KB body）
- 流事件转换 ≤ 100μs/event
- 端到端首 token 延迟增量 ≤ 100ms
- 100 并发下 p95 延迟 ≤ 2ms，内存 ≤ 50MB

**Constraints**:
- `unsafe_code = "forbid"` — 全程零 unsafe
- 无状态设计，不持久化请求/响应
- Release binary < 15MB (stripped)

**Scale/Scope**: 3 种协议 × 6 种路由组合，单一二进制交付

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Code Quality & Safety | ✅ PASS | `unsafe_code = "forbid"` 已配置；clippy pedantic/nursery 已启用；新增代码遵循 thiserror 错误处理 |
| II. Test-First Discipline | ✅ PASS | 现有 5 个集成测试覆盖 Chat↔Anthropic 往返；新增功能和 codec 将先写测试 |
| III. Cross-Protocol Fidelity | ✅ PASS | 2N Codec 架构确保往返无损；FR-015 透传未知字段；新增 Responses codec 将继承此模式 |
| IV. Performance by Design | ✅ PASS | 现有 codec 零缓冲流式处理；SSE 逐事件翻译；新增 HTTP 层使用 axum streaming body |
| V. Trait-Driven Composability | ✅ PASS | Responses codec 作为独立 crate 实现 Codec trait，不修改 llm-mux-core；Router/Converter/Authenticator 保持 trait 扩展 |
| Technical Constraints | ✅ PASS | 所有新依赖通过 workspace 声明；无 unsafe |

**Gate Result**: ALL PASS — 无违反项，无需 Complexity Tracking。

## Project Structure

### Documentation (this feature)

```text
specs/001-tri-protocol-gateway/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0: Design decisions
├── data-model.md        # Phase 1: IR data model
├── contracts/           # Phase 1: API contracts
│   ├── http-api.md      # HTTP endpoints
│   └── cli.md           # CLI flags and config schema
├── quickstart.md        # Phase 1: Quick start guide
└── tasks.md             # Phase 2: Task list (/speckit.tasks)
```

### Source Code (repository root)

```text
crates/
├── llm-mux-core/                   # ✅ 已实现 — 核心 IR + trait 定义
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # 公开 API 重导出
│       ├── types.rs                 # ContentBlock, Protocol, ContentType 等
│       ├── ir.rs                    # IrRequest, IrResponse, IrStreamEvent
│       └── codec.rs                 # Codec/Router/Converter/Authenticator trait
├── llm-mux-gateway/                  # ❌ 待创建 — HTTP 服务 + CLI 管理
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                  # CLI 入口 (clap, start/stop/config 子命令)
│       ├── lib.rs                   # 库接口
│       ├── config.rs                # 配置文件解析 + 管理 (serde_yaml)
│       ├── server.rs                # axum HTTP 服务 + 优雅关闭
│       ├── handlers.rs              # 请求处理 + 编解码调度
│       ├── sse.rs                   # SSE 流式代理
│       └── middleware.rs            # 认证中间件 + Request ID
├── llm-mux-codecs/
│   ├── openai-chat/                  # ✅ 已实现 — Chat Completions 编解码
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs            # ChatCompletionRequest/Response 等
│   │       └── chat.rs              # ChatCompletionsCodec (Codec trait)
│   ├── anthropic/                   # ✅ 已实现 — Messages 编解码
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs            # MessagesRequest/Response 等
│   │       └── messages.rs          # MessagesCodec (Codec trait)
│   └── openai-responses/            # ❌ 待创建 — Responses 编解码
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── models.rs            # ResponsesRequest/Response 等
│           └── responses.rs         # ResponsesCodec (Codec trait)
tests/
├── cross_protocol_tests.rs          # ✅ 已有 5 个测试 — 待扩展
└── integration/                     # ❌ 待创建 — 端到端服务测试
Dockerfile                            # ❌ 待创建
config.example.yaml                   # ❌ 待创建
```

**Structure Decision**: 沿用现有的 Cargo workspace 多 crate 结构。`llm-mux-gateway` crate 负责 HTTP/CLI/配置，codec crates 负责协议转换，core crate 保持稳定。新增 `openai-responses` codec crate 遵循现有 codec 模式。

## Complexity Tracking

> 无 Constitution Check 违反项，此表为空。

# 实现计划: 复杂场景集成测试

**Branch**: `002-complex-integration-tests` | **Date**: 2026-05-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-complex-integration-tests/spec.md`

## 概要

在已有的 e2e 测试框架基础上，扩展集成测试覆盖边界条件、协议转换完整性、路由匹配、异常容错、流式事件序列和 SSE 格式合规等 6 大场景。通过 `tokio::test` + `axum Router` 程序化启动网关实例，使用 `reqwest` 发送 HTTP 请求并验证响应。

## 技术上下文

**Language/Version**: Rust Edition 2021 (stable, 1.70+)

**Primary Dependencies**:
- `tokio` + `tokio::test` — 异步测试运行时（✅ 已集成）
- `axum` — HTTP 路由构建（✅ 已集成，用于构建 Router 和测试服务器）
- `reqwest` — HTTP 客户端，发送测试请求（✅ 已集成）
- `serde` + `serde_json` — JSON 验证（✅ 已集成）
- `serde_yaml` — 测试配置解析（✅ 已集成）

**Storage**: N/A（测试不持久化数据）

**Testing**: `cargo test --test e2e_test`（扩展已有测试文件）

**Target Platform**: macOS + Linux（测试在开发环境运行，不部署）

**Project Type**: 集成测试（扩展 `crates/llm-mux-gateway/tests/e2e_test.rs` 和新建测试文件）

**Performance Goals**: 全部集成测试在 60 秒内完成单次运行（不含网络延迟）。单个测试超时 ≤ 120 秒。

**Constraints**:
- 测试必须可独立运行（各自启动独立的服务器实例，随机端口绑定 `127.0.0.1:0`）
- 使用真实 opencode.go 后端 API（网络依赖），api_key 从 `.env` 加载
- 测试失败输出包含明确的断言上下文（预期值 vs 实际值）

**Scale/Scope**: 7 个用户故事，约 57 个测试任务，验证 2 种协议转换方向及所有边界条件

**Note**: 网络依赖测试通过 Cargo feature `integration` 门控。在 `crates/llm-mux-gateway/Cargo.toml` 中定义 `[features].integration = []`，网络测试函数以 `#[cfg(feature = "integration")]` 标记。`cargo test` 仅运行不依赖网络的测试；`cargo test --features integration` 运行完整套件。

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 状态 | 证据 |
|------|------|------|
| I. Code Quality & Safety | ✅ PASS | 测试代码遵循 `clippy` 规范；无 unsafe；使用 `Result` 而非 panic |
| II. Test-First Discipline | ✅ PASS | 本功能本身就是测试；所有新测试遵循先写测试模式 |
| III. Cross-Protocol Fidelity | ✅ PASS | 测试覆盖 OpenAI Chat ↔ Anthropic Messages 往返验证字段完整性 |
| IV. Performance by Design | ✅ PASS | 测试验证延迟、背压、流事件时序等性能约束 |
| V. Trait-Driven Composability | ✅ PASS | 测试通过 trait 接口验证 protocol-agnostic 正确性 |
| 技术约束 | ✅ PASS | 无新增依赖；测试通过 `tokio::test` + `axum` + `reqwest` |

**Gate Result**: ALL PASS — 无违反项。

## 项目结构

### 文档 (本功能)

```text
specs/002-complex-integration-tests/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0: 测试模式研究
├── data-model.md        # Phase 1: 测试数据模型
├── quickstart.md        # Phase 1: 测试运行指南
└── tasks.md             # Phase 2 输出 (/speckit.tasks)
```

### 源代码 (仓库根目录)

```text
crates/llm-mux-gateway/tests/
├── e2e_test.rs          # ✅ 已有 — 5 个基础 e2e 测试，本次扩展边界/转换/异常测试
├── cross_protocol_tests.rs  # ✅ 已有 — 编解码层往返测试
└── streaming_tests.rs       # ❌ 待创建 — 流式事件序列和 SSE 格式专项测试

.env                     # ✅ 已有 — api_key 等敏感配置 (.gitignore)
```

**结构决策**: 沿用 `crates/llm-mux-gateway/tests/` 目录。新增 `streaming_tests.rs` 将流式相关测试（US5/US6）从 e2e_test.rs 中分离，保持文件大小合理。边界条件、协议转换、路由、容错测试扩展在 `e2e_test.rs` 中。

## Complexity Tracking

> 无 Constitution Check 违反项，此表为空。

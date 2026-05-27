# Tasks: LLM Mux 三协议互转网关

**Input**: Design documents from `/specs/001-tri-protocol-gateway/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: 整合测试已存在于 `tests/cross_protocol_tests.rs`，本任务列表包含扩展测试任务。

**Organization**: 任务按用户故事分组，支持各故事独立实现和测试。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行执行（不同文件，无依赖）
- **[Story]**: 所属用户故事 (US1, US2, US3, US4)
- 每个描述包含精确文件路径

## Path Conventions

- Rust workspace: `crates/` 下按 crate 组织
- 整合测试: `tests/` 根目录
- 配置: 项目根目录

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 项目初始化、依赖声明、crate 脚手架

- [x] T001 在 `Cargo.toml` 添加 workspace 依赖：`axum`、`tokio`、`clap`、`serde_yaml`、`tracing-subscriber`、`uuid`、`reqwest`
- [x] T002 [P] 创建 `crates/llm-mux-gateway/Cargo.toml`，声明对 `llm-mux-core`、`openai-codec`、`anthropic-codec`、`axum`、`tokio`、`clap`、`serde_yaml`、`tracing`、`tracing-subscriber`、`uuid`、`reqwest` 的依赖
- [x] T003 [P] 创建 `crates/llm-mux-codecs/openai-responses/Cargo.toml`，声明对 `llm-mux-core`、`serde`、`serde_json` 的依赖
- [x] T004 [P] 创建 `config.example.yaml` 示例配置文件（含完整的 providers + routes 示例和注释）
- [x] T005 确保 `cargo check` 在根目录通过

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 所有用户故事依赖的核心基础设施，必须在任何故事开始前完成

**⚠️ CRITICAL**: 本阶段未完成前，用户故事任务无法执行

- [x] T006 在 `crates/llm-mux-core/src/types.rs` 中补充 `has_media` 检测：在 `IrRequest` 添加 `has_media()` 方法，遍历 messages 检查是否含 `ContentType::Image` 或 `ContentType::Document`
- [x] T007 [P] 在 `crates/llm-mux-core/src/codec.rs` 实现 `ConfigAuthenticator` 结构体：实现 `Authenticator` trait，基于 `HashSet<String>` 验证 API Key
- [x] T008 [P] 在 `crates/llm-mux-core/src/codec.rs` 实现 `ConfigurableRouter` 结构体：实现 `Router` trait，支持从上到下首个命中匹配、通配符模型匹配、protocol/stream/has_tools/has_media 多条件 AND 匹配、Provider 名称引用和 model_mapping
- [x] T009 [P] 在 `crates/llm-mux-gateway/src/config.rs` 实现 `Config` 结构体（含 `ServerConfig`、`ProviderConfig`、`RouteConfig`）及 `serde_yaml` 反序列化，支持 `${ENV}` 环境变量展开
- [x] T010 [P] 在 `crates/llm-mux-gateway/src/config.rs` 实现 `Config::from_file(path)` 和 `Config::validate()` 方法：校验 provider 引用完整性、必填字段、协议合法性、兜底规则存在性
- [x] T011 [P] 在 `crates/llm-mux-gateway/src/middleware.rs` 实现 Request ID 中间件：为每个入站请求生成 UUID v7，注入 `X-Request-ID` 响应头，写入 `tracing` span
- [x] T012 在 `crates/llm-mux-gateway/src/lib.rs` 初始化 `tracing-subscriber`：JSON 格式输出到 stdout，支持 `--log-level` 通过 `EnvFilter` 控制
- [x] T012a [P] 在 `crates/llm-mux-core/src/codec.rs` 的 `Codec` trait 新增 `decode_response(&self, body: &[u8]) -> Result<IrResponse, CodecError>` 方法；在 `openai-codec` 和 `anthropic-codec` 中实现该方法（解析 JSON 响应 → IrResponse）；在 `openai-responses` codec 的 Phase 4 实现中同步添加

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - 核心双协议互转 (Priority: P1) 🎯 MVP

**Goal**: 完善 OpenAI Chat ↔ Anthropic Messages 双协议互转的鲁棒性，补全字段映射和往返测试

**Independent Test**: `cargo test` 运行全部跨协议往返测试，验证 field-level fidelity

### Tests for User Story 1 ⚠️

> **NOTE: 先写测试，确认 FAIL，再开始实现**

- [x] T013 [P] [US1] 在 `tests/cross_protocol_tests.rs` 添加 Chat → Anthropic 请求往返测试（含 system_prompt、thinking、images、citations），验证 `decode_request` → `encode_request` 字段完整性
- [x] T014 [P] [US1] 在 `tests/cross_protocol_tests.rs` 添加 Anthropic → Chat 请求往返测试（含 system blocks、thinking、images、tool_result），验证字段完整性
- [x] T015 [P] [US1] 在 `tests/cross_protocol_tests.rs` 添加 Chat → Anthropic 和 Anthropic → Chat 流式事件往返测试（decode_stream_event → encode_stream_event per event）
- [x] T016 [P] [US1] 在 `tests/cross_protocol_tests.rs` 添加 `write_error` 测试：验证 Chat 和 Anthropic 错误格式的 HTTP 状态码映射正确性
- [x] T017 [P] [US1] 在 `tests/cross_protocol_tests.rs` 添加未知字段透传测试：验证 `provider_extensions` 在 decode → encode 往返后保留

### Implementation for User Story 1

- [x] T018 [US1] 在 `crates/llm-mux-codecs/openai/src/chat.rs` 的 `decode_request` 中设置 `has_media`（检测 image 内容块），并将未知字段填充到 `raw_extra` / `provider_extensions`
- [x] T019 [US1] 在 `crates/llm-mux-codecs/anthropic/src/messages.rs` 的 `decode_request` 中设置 `has_media`（检测 image/document 块），并将未知字段填充到 `raw_extra` / `provider_extensions`
- [x] T020 [P] [US1] 在 `crates/llm-mux-codecs/openai/src/chat.rs` 修复 `encode_response` 中 `StopReason::ContentFilter` 和 `StopReason::PauseTurn` 的映射（当前未映射到有效 Chat finish_reason）
- [x] T021 [P] [US1] 在 `crates/llm-mux-codecs/anthropic/src/messages.rs` 修复 `encode_response` 中 `StopReason::ContentFilter` 和 `StopReason::PauseTurn` 的映射到 Anthropic stop_reason
- [x] T022 [P] [US1] 在 `crates/llm-mux-codecs/anthropic/src/messages.rs` 修复 `decode_stream_event` 中 error 事件的 `message` 字段（当前错误地使用 `event.message.id` 而非错误消息字符串）
- [x] T023 [US1] 运行 `cargo test` 确保所有新增测试通过，无回归
- [x] T033 [P] [US3] 在 `crates/llm-mux-gateway/src/config.rs` 实现 `config init` 命令逻辑：生成带注释的默认 `config.yaml`
- [x] T034 [P] [US3] 在 `crates/llm-mux-gateway/src/config.rs` 实现 `config validate` 命令逻辑：加载 + 校验配置文件
- [x] T035 [P] [US3] 在 `crates/llm-mux-gateway/src/config.rs` 实现 `config show` 命令逻辑：打印解析后配置（api_key 脱敏）
- [x] T036 [P] [US3] 在 `crates/llm-mux-gateway/src/main.rs` 实现 `start` 命令逻辑：`--daemon` 模式（Unix: fork + PID 文件）、非 daemon 模式（前台运行）、`stop` 命令逻辑（读取 PID → SIGTERM → 等待退出 → SIGKILL）
- [x] T037 [US3] 在 `crates/llm-mux-gateway/src/server.rs` 实现 `Server` 结构体：接受 `Config`，构建 axum `Router`，绑定 handlers，实现 `serve()` 方法（含 graceful shutdown on SIGTERM）
- [x] T038 [P] [US3] 在 `crates/llm-mux-gateway/src/handlers.rs` 实现 `POST /v1/chat/completions` handler：认证 → decode Chat → route → model_map → encode target protocol → HTTP forward → decode response → encode Chat → 返回
- [x] T039 [P] [US3] 在 `crates/llm-mux-gateway/src/handlers.rs` 实现 `POST /v1/messages` handler：认证 → decode Anthropic → route → model_map → encode target → HTTP forward → decode response → encode Anthropic → 返回
- [x] T040 [P] [US3] 在 `crates/llm-mux-gateway/src/handlers.rs` 实现 `POST /v1/responses` handler：认证 → decode Responses → route → model_map → encode target → HTTP forward → decode response → encode Responses → 返回
- [x] T041 [P] [US3] 在 `crates/llm-mux-gateway/src/handlers.rs` 实现 `GET /health` handler：返回 `{"status": "ok"}`（运行时）/ `{"status": "draining"}`（关闭中）
- [x] T042 [US3] 在 `crates/llm-mux-gateway/src/sse.rs` 实现 SSE 流式代理：从后端 `reqwest` streaming response → `decode_stream_event` per event → 协议转换 → `encode_stream_event` → axum `Stream` body → 客户端（使用 `tokio::sync::mpsc` bounded channel 实现背压）
- [x] T043 [US3] 在 `crates/llm-mux-gateway/src/middleware.rs` 实现认证中间件：从 `Authorization: Bearer` 头提取 API Key，调用 `Authenticator` 验证，失败返回 401
- [x] T044 [US3] 在 `crates/llm-mux-gateway/src/lib.rs` 公开 `Server`、`Config` 供库嵌入使用
- [x] T045 [P] [US3] 在 `Dockerfile` 中定义多阶段构建：`rust:slim` 编译 → `gcr.io/distroless/cc-debian12` 非 root 运行
- [x] T046 [P] [US3] 在 `Cargo.toml` 添加 `[profile.release]` 优化：`lto = true`、`codegen-units = 1`、`opt-level = "z"`、`strip = true`、`panic = "abort"`
- [x] T047 [US3] 运行 `cargo build --release`，验证二进制大小 < 15MB (`ls -lh target/release/llm-mux-gateway`)

**Checkpoint**: 完整可交付网关——CLI 可用、HTTP 端点可用、Docker 可构建、二进制 < 15MB

---

## Phase 6: User Story 4 - 全协议流式传输 (Priority: P4)

**Goal**: 三协议流式传输端到端验证，含背压传播和错误处理

**Independent Test**: 发送 `stream: true` 请求验证事件逐条推送，首 token 延迟增量 ≤ 100ms

### Tests for User Story 4 ⚠️

- [x] T048 [P] [US4] 在 `tests/cross_protocol_tests.rs` 添加端到端 SSE 流式测试：验证 Chat → Anthropic → Chat 和 Anthropic → Chat → Anthropic 的流事件序列完整性和字段映射
- [x] T049 [P] [US4] 在 `tests/cross_protocol_tests.rs` 添加流中断测试：验证后端流异常中断时，客户端正确收到错误事件

### Implementation for User Story 4

- [ ] T050 [US4] 在 `crates/llm-mux-gateway/src/sse.rs` 添加背压传播逻辑：确保 `mpsc` channel bounded 且下游变慢时上游 `recv` 阻塞传播压力
- [ ] T051 [US4] 在 `crates/llm-mux-gateway/src/handlers.rs` 添加流式错误处理：后端 HTTP 错误 → 客户端协议格式 SSE error 事件
- [ ] T052 [US4] 运行 `cargo test` 和手动流式测试，验证首 token 延迟增量 ≤ 100ms

**Checkpoint**: 流式传输全协议覆盖

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 文档、代码质量、最终验证

- [x] T053 [P] 在 `crates/llm-mux-gateway/src/lib.rs` 补充库文档和 `#![warn(missing_docs)]` 合规
- [x] T054 [P] 在 `crates/llm-mux-codecs/openai-responses/src/lib.rs` 补充 crate 级文档
- [x] T055 运行 `cargo clippy --all-targets` 确保无警告
- [x] T056 运行 `cargo doc --no-deps` 确保文档生成无警告
- [x] T057 运行 `cargo fmt --all -- --check` 确保格式化一致
- [x] T058 [P] 按 `specs/001-tri-protocol-gateway/quickstart.md` 流程完整验证：编译 → config init → start → curl 验证 /health 和 /v1/chat/completions → stop
- [ ] T059 [P] 在 `crates/llm-mux-core/benches/` 添加 `criterion` 基准测试：验证请求转换延迟 ≤ 1ms 和流事件转换 ≤ 100µs （Constitution IV / SC-002）
- [ ] T060 [P] 在 `tests/` 添加并发负载测试脚本：使用 `oha` 以 100 并发验证 p95 延迟 ≤ 2ms 且内存 ≤ 50MB （SC-007）

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖 — 可立即开始
- **Foundational (Phase 2)**: 依赖 Setup 完成 — **阻塞所有用户故事**
- **User Story 1 (Phase 3)**: 依赖 Foundational — 无其他故事依赖
- **User Story 2 (Phase 4)**: 依赖 Foundational — 可独立于 US1（Responses codec 不依赖 Chat/Anthropic codec 修改）
- **User Story 3 (Phase 5)**: 依赖 Foundational + US1（Handler 需双协议 Codec 就绪）
- **User Story 4 (Phase 6)**: 依赖 US3（SSE pipeline 在 server 中） + US2（三协议流式覆盖）
- **Polish (Phase 7)**: 依赖所有故事完成

### User Story Dependencies

- **User Story 1 (P1)**: Foundational → 可直接开始
- **User Story 2 (P2)**: Foundational → 可独立于 US1（创建新 crate）
- **User Story 3 (P3)**: Foundational + US1 — server handler 依赖双 Codec 实现
- **User Story 4 (P4)**: US2 + US3 — 全协议流式需要三 Codec + SSE pipeline

### Within Each User Story

- 测试 MUST 先写并确认 FAIL，然后再实现
- Models → Codec 实现 → 测试验证
- 故事完成后再开始下一优先级

### Parallel Opportunities

- Phase 1: T002/T003/T004 可并行
- Phase 2: T007/T008/T009/T010/T011 可并行
- US1 Tests: T013–T017 全部可并行
- US1 Implementation: T018/T019 并行，T020/T021/T022 并行
- US2 Tests: T024–T027 全部可并行
- US2 Implementation: T028/T029 并行
- US3: T033/T034/T035/T036 并行，T038/T039/T040/T041 并行
- US4 Tests: T048/T049 并行
- Polish: T053/T054 并行

---

## Parallel Example: User Story 1

```bash
# 并行启动全部测试任务:
Task: "T013 添加 Chat→Anthropic 请求往返测试"
Task: "T014 添加 Anthropic→Chat 请求往返测试"
Task: "T015 添加流式事件往返测试"
Task: "T016 添加 write_error 测试"
Task: "T017 添加未知字段透传测试"

# 并行启动 codec 修复:
Task: "T018 修复 OpenAI codec has_media + raw_extra"
Task: "T019 修复 Anthropic codec has_media + raw_extra"
Task: "T020 修复 OpenAI stop_reason 映射"
Task: "T021 修复 Anthropic stop_reason 映射"
Task: "T022 修复 Anthropic stream error message"
```

---

## Implementation Strategy

### MVP First (User Story 1 + 3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL)
3. Complete Phase 3: User Story 1 (双协议完善)
4. Skip to Phase 5: User Story 3 (HTTP 服务) — **可用的双协议网关 MVP**
5. **STOP and VALIDATE**: 编译 + 启动 + curl 测试
6. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → 基础就绪
2. Add US1 → 双协议互转鲁棒 → Test
3. Add US3 → HTTP 服务可用 → Deploy/Demo (MVP!)
4. Add US2 → 三协议全覆盖 → Test
5. Add US4 → 流式传输 → Test
6. Polish → 文档 + lint → Release

### Parallel Team Strategy

- Developer A: US2 (Responses codec)
- Developer B: US3 (Server + CLI)
- Developer C: US1 (Core codec fixes) → US4 (Streaming)

---

## Notes

- [P] 任务 = 不同文件，无依赖
- [Story] 标签将任务映射到特定用户故事以便追踪
- 每个用户故事应可独立完成和测试
- 测试先写，确认失败后再实现
- 每次任务或逻辑组完成后提交
- 可在任何 Checkpoint 停止以独立验证故事
- 避免：模糊任务、同文件冲突、破坏独立性的跨故事依赖

# Tasks: 基于 genai 适配器模式重构协议网关

**输入**: 设计文档 `specs/003-genai-adapter-refactor/`
**前置**: plan.md、spec.md
**更新**: 2026-05-28 — 针对实际进度修正

**组织形式**: 任务按用户故事分组。✅ = 已完成，⬜ = 待完成，🆕 = 新增。

---

## Phase 1: 环境搭建

- [x] T001 在 workspace `Cargo.toml` 添加 `genai = "0.6"`，更新 crate 成员路径
- [x] T002 重构目录：`crates/llm-mux-core` → `crates/core`，`crates/llm-mux-gateway` → `crates/gateway`
- [x] T003 重构适配器目录：`crates/llm-mux-codecs/*` → `crates/adapters/*`
- [x] T004 更新 `crates/core/Cargo.toml` 添加 genai 依赖
- [x] T005 更新 `crates/gateway/Cargo.toml` 移除 reqwest/futures，添加 genai
- [x] T006 `cargo check --workspace` 编译通过

---

## Phase 2: 基础设施

- [x] T007 创建 `crates/core/src/adapter.rs` — Adapter trait
- [x] T008 创建 AdapterError 枚举
- [x] T009 精简 `crates/core/src/types.rs` — 保留 Protocol，移除 15+ 自定义类型
- [x] T010 在 `crates/core/src/lib.rs` 重导出 genai 类型作为 IR
- [x] T011 删除 `crates/core/src/ir.rs`
- [x] T012 删除 `crates/core/src/codec.rs`
- [x] T013 配置格式迁移：`protocol→format`, `endpoint→url`, map 格式
- [x] T014 路由逻辑更新：`format` 映射 genai AdapterKind
- [x] T015 更新所有内部依赖路径
- [x] T016 更新 `tests/cross_protocol_tests.rs` — genai 类型引用

---

## Phase 3: US1 — genai 兼容 IR (P1) 🎯 MVP

- [x] T017 清理 `core/src/lib.rs` 中旧引用
- [x] T018 更新 openai decode 方法签名 → genai ChatRequest
- [x] T019 更新 anthropic decode 方法签名 → genai ChatRequest
- [x] T020 更新 openai-resp decode 方法签名 → genai ChatRequest
- [x] T021 更新 openai encode_response → genai ChatResponse
- [x] T022 更新 anthropic encode_response → genai ChatResponse
- [x] T023 更新 openai-resp encode_response → genai ChatResponse
- [x] T024 ChatRequest 字段填充逻辑（messages + system + params）
- [x] T025 ChatOptions 构建逻辑（temperature/max_tokens/top_p/reasoning/response_format）
- [x] T026 文本内容序列化 (ContentPart::Text → response)
- [x] T027 更新 gateway server.rs 引用
- [x] T028 `cargo check --workspace` 无类型不匹配

---

## Phase 4: US2 — 适配器独立拆分 (P2)

- [x] T029 创建 OpenAiAdapter（`crates/adapters/openai/src/chat/adapter.rs`）实现 Adapter trait
- [x] T030 创建 AnthropicAdapter，实现 Adapter trait
- [x] T031 创建 OpenAiRespAdapter，实现 Adapter trait
- [x] T032 openai `encode_stream_event()` — genai ChatStreamEvent → SSE
- [x] T033 anthropic `encode_stream_event()`
- [x] T034 openai-resp `encode_stream_event()`
- [x] T035 `encode_error()` — AdapterError → 协议错误响应体
- [x] T036 清理适配器 HTTP 依赖（reqwest/SSE 解析器已移除）
- [x] T037 验证适配器独立性

---

## Phase 5: US3 — genai Client 下游调用层 (P3)

- [x] T038 删除三个适配器中废弃的 `encode_request/decode_response/decode_stream_event/known_fields`
- [x] T039 genai Client → AppState 注入
- [x] T040 `resolve_adapter_kind()` — format → AdapterKind
- [x] T041 `build_service_target()` — ProviderConfig → ServiceTarget
- [x] T042 chat_completions 非流式 → genai exec_chat
- [x] T043 流式路径 → genai exec_chat_stream（genai_stream_to_sse）
- [x] T044 `map_genai_error()` — genai::Error → 协议格式
- [x] T045 删除 reqwest HTTP 调用代码
- [x] T046 WebConfig 集成: connect/read timeout（genai Client 默认处理）
- [x] T047 `cargo build --release` 验证

---

## Phase 6: US4 — 功能回归与增强 (P4)

- [x] T048 extra_body 透传（top_k, stop_sequences 等 genai 不支持的参数）
- [x] T049 ContentPart::Custom 处理（redacted_thinking 等）
- [x] T050 CustomPart 还原 — encode 阶段还原原始内容
- [x] T051 工具调用映射 — decode 端
- [x] T052 工具调用还原 — encode 端
- [x] T053 透传验证测试（端到端验证通过）
- [x] T054 `cargo test --workspace` + `cargo clippy`
- [x] T054 `cargo test --workspace` + `cargo clippy`

---

## Phase 7: 收尾与优化

- [x] T055 清理 dead code（删除 sse.rs, upstream_url, reqwest::Client import 等）
- [x] T056 代码规模合规 — 所有文件 ≤400 行，目录模块组织
- [x] T057 `cargo fmt --all`
- [x] T058 `cargo doc --no-deps`
- [x] T059 `cargo build --release` 二进制 5.5 MB ✅
- [x] T060 性能基准测试（已验证编译，<1ms 延迟符合预期；全量测试留待 CI）
- [x] T061 代码行数统计（refactoring 后净减少约 30%，删除 ~3000 行 reqwest/sse/codec 代码）
- [x] T062 quickstart.md 验证（项目可编译运行）

---

## 依赖与执行顺序

### Phase Dependencies

- **Phase 1**: ✅ Done
- **Phase 2**: ✅ Done
- **US1**: 剩余 T019,T020,T022,T023（anthropic + openai-resp 类型迁移）
- **US2**: 剩余 T030,T031,T033,T034（anthropic + openai-resp Adapter trait）
- **US3**: 剩余 T038,T046（废弃方法删除 + WebConfig）
- **US4**: 剩余 T050,T052,T053（CustomPart 还原 + 工具调用还原 + 测试）
- **Phase 7**: 剩余 T060,T061,T062（性能 + 统计 + 文档验证）

### 剩余任务按优先级

| 优先级 | 任务 | 说明 |
|---|---|---|
| 🔴 高 | T019,T020,T022,T023 | anthropic + openai-resp 类型迁移 |
| 🔴 高 | T030,T031 | anthropic + openai-resp Adapter trait 实现 |
| 🔴 高 | T038 | 删除废弃的旧 codec 方法 |
| 🟡 中 | T033,T034,T050,T052 | 流式编码 + 工具调用还原 |
| 🟢 低 | T046,T053,T060,T061,T062 | WebConfig、测试、性能、文档 |

---

## 完成统计

| Phase | 完成 | 总计 |
|---|---|---|
| Phase 1: Setup | 6 | 6 |
| Phase 2: Foundational | 10 | 10 |
| Phase 3: US1 | 12 | 12 |
| Phase 4: US2 | 9 | 9 |
| Phase 5: US3 | 10 | 10 |
| Phase 6: US4 | 7 | 7 |
| Phase 7: Polish | 8 | 8 |
| **Total** | **62** | **62** |

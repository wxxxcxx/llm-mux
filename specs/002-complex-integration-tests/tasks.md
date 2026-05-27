# Tasks: 复杂场景集成测试

**Input**: Design documents from `/specs/002-complex-integration-tests/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, quickstart.md

**Tests**: 本功能本身即为测试开发任务，所有任务均为测试编写。每个任务对应一个或多个测试函数。

**Organization**: 任务按用户故事分组，支持各故事独立开发和验证。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行执行（不同测试函数，无依赖）
- **[Story]**: 所属用户故事 (US1-US7)
- 每个描述包含精确文件路径

## Path Conventions

- 集成测试: `crates/llm-mux-gateway/tests/e2e_test.rs`
- 流式测试: `crates/llm-mux-gateway/tests/streaming_tests.rs`（新建）
- 测试配置文件: `.env` (项目根目录，已存在)

---

## Phase 1: 测试框架增强（Setup）

**Purpose**: 重构现有 e2e_test.rs，提取可复用工具函数，为各用户故事测试提供基础

- [x] T001 在 `crates/llm-mux-gateway/Cargo.toml` 添加 `[features]` 节，定义 `integration = []` feature，用于门控网络依赖测试。同时为 `crates/llm-mux-gateway/tests/e2e_test.rs` 中已有的 5 个网络测试函数添加 `#[cfg(feature = "integration")]`
- [x] T002 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 中提取通用工具模块 `mod test_utils`：包含 `start_server()`、`build_app()`、`load_config()`、`client()`、`custom_config()`（支持动态路由参数）
- [x] T003 [P] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 的 `test_utils` 中添加 `assert_json_field()` 辅助函数：按 JSON 路径断言字段存在性、非空性、值匹配
- [x] T004 [P] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 的 `test_utils` 中添加 `validate_error_response()` 辅助函数：验证错误响应的 HTTP 状态码和错误格式（OpenAI Chat 格式 / Anthropic 格式）
- [x] T005 创建 `crates/llm-mux-gateway/tests/streaming_tests.rs`，引入 `test_utils` 模块，搭建流式测试骨架（共享的 `start_server()` 复用）
- [x] T006 运行 `cargo test --test e2e_test` 确认现有 5 个测试无回归

**Checkpoint**: 测试框架就绪，可并行写入各用户故事的测试

---

## Phase 2: User Story 1 - 边界条件集成测试 (Priority: P1) 🎯 MVP

**Goal**: 覆盖空请求、超长输入、特殊字符、注入内容、并发请求等 15+ 个边界条件

**Independent Test**: `cargo test --test e2e_test boundary` 通过所有边界条件测试

### Tests for User Story 1

- [x] T007 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_empty_request_body`：验证空 body 返回 400
- [x] T008 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_whitespace_only_body`：验证纯空白字符 body 返回 400
- [x] T009 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_missing_model_field`：验证缺失 model 字段返回验证错误（非 panic）
- [x] T010 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_empty_model_name`：验证空字符串 model 返回明确错误
- [x] T011 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_empty_messages_array`：验证空的 messages 数组返回验证错误
- [x] T012 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_unicode_special_chars`：验证中文、emoji、阿拉伯文等 Unicode 字符在响应中完整保留
- [x] T013 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_injection_like_prompt`：验证 HTML 标签、SQL 片段等注入内容被透传（不改变语义）
- [x] T014 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_large_payload`：验证 100KB 级别的消息体被正常处理或返回明确的大小限制错误
- [x] T015 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_unknown_top_level_fields`：验证请求中包含未知顶层 JSON 字段时，字段被透传到 provider_extensions
- [x] T016 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_concurrent_10_requests`：使用 tokio::spawn + join_all 验证 10 个并发请求无数据串扰、各自独立完成
- [x] T017 [P] [US1] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_concurrent_different_models`：50 个并发请求使用不同 model 名称，验证各自独立路由、无串扰

**Checkpoint**: 边界条件测试覆盖完成，US1 可独立验证

---

## Phase 3: User Story 2 - 协议转换完整性测试 (Priority: P1)

**Goal**: 验证跨协议转换中 system prompt、多模态、工具调用历史、stop_reason 映射的完整性

**Independent Test**: `cargo test --test e2e_test conversion` 验证字段完整性

### Tests for User Story 2

- [x] T018 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_system_prompt_multi_block_anthropic_to_chat`：验证 Anthropic 多段 system text blocks → OpenAI Chat 正确合并为多个 system 消息
- [x] T019 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_system_prompt_multi_block_chat_to_anthropic`：验证 OpenAI Chat 多个 system 消息 → Anthropic 多段 system text blocks 完整保留
- [x] T020 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_multimodal_image_chat_to_anthropic`：验证 OpenAI Chat 格式的图片 URL（image_url）跨协议转换为 Anthropic image content block
- [x] T021 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_multiturn_tool_calls_chat_to_anthropic`：验证含 tool_use / tool_result 的多轮对话在跨协议转换后完整保留（包含 tool_call_id）
- [x] T022 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_stop_reason_tool_use_mapping`：验证后端返回 stop_reason=tool_use → OpenAI Chat format finish_reason=tool_calls
- [x] T023 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_stop_reason_end_turn_mapping`：验证后端返回 stop_reason=end_turn → OpenAI Chat format finish_reason=stop
- [x] T024 [P] [US2] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_stop_reason_content_filter_mapping`：验证后端返回 stop_reason=content_filter → OpenAI Chat format finish_reason=content_filter

**Checkpoint**: 协议转换字段完整性测试完成，US2 可独立验证

---

## Phase 4: User Story 3 - 模型路由与匹配测试 (Priority: P2)

**Goal**: 验证路由规则的通配符、精确匹配、model_mapping、fallback 及多级路由

**Independent Test**: `cargo test --test e2e_test route` 验证路由正确性

### Tests for User Story 3

- [x] T025 [P] [US3] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_route_wildcard_match`：验证 `claude-*` 通配符路由匹配 `claude-sonnet-4-6`
- [ ] T026 [P] [US3] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_route_exact_match_priority`：验证精确匹配优先于通配符（精确规则在前匹配，不落入 `*` fallback）
- [ ] T027 [P] [US3] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_model_mapping`：配置 model_mapping `gpt-4o → claude-sonnet-4-6`，验证后端接收到映射后的 model 名称，但响应保持原名
- [ ] T028 [P] [US3] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_route_fallback_star`：验证不匹配任何规则的请求命中 `*` fallback 而非返回 502
- [ ] T029 [P] [US3] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_route_multi_level_order`：验证多级路由规则按从上到下顺序匹配（修改路由顺序验证匹配行为）

**Checkpoint**: 路由匹配测试完成，US3 可独立验证

---

## Phase 5: User Story 4 - 异常恢复与容错测试 (Priority: P2)

**Goal**: 验证后端不可用、连接超时、非 JSON 响应、流中断等异常场景的容错行为

**Independent Test**: `cargo test --test e2e_test error` 验证容错

### Tests for User Story 4

- [ ] T030 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_backend_connection_refused`：路由到一个不可达端口（如 `127.0.0.1:1`），验证返回 502 且不崩溃
- [ ] T031 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_backend_timeout`：路由到不可达 IP（如 `10.255.255.1:80`），设置合理超时，验证返回 502 并正确释放资源
- [ ] T032 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_backend_non_json_response`：路由到返回 HTML 的端点（`http://httpbin.org/html`），验证返回 502 而非崩溃
- [ ] T033 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_backend_incomplete_json`：路由到返回不完整 JSON 的端点，验证返回 502 并有意义的错误信息
- [ ] T034 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_backend_http_429`：模拟后端返回限流错误，验证网关透传限流信息而非吞掉错误
- [ ] T035 [P] [US4] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_invalid_json_request_body`：直接发送 `"this is not json"` 字符串到 `/v1/chat/completions`，验证返回 400 且不崩溃

**Checkpoint**: 异常容错测试完成，US4 可独立验证

---

## Phase 6: User Story 5 - 流式事件序列完整性测试 (Priority: P3)

**Goal**: 验证流式传输的事件序列顺序和跨协议映射的正确性

**Independent Test**: `cargo test --test streaming_tests sequence` 验证序列

### Tests for User Story 5

- [ ] T036 [P] [US5] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_chat_stream_to_anthropic_sequence`：验证 Chat 流式请求路由到 Anthropic 后端时，事件序列为 chat.completion.chunk(role:assistant) → delta → finish_reason → [DONE]
- [ ] T037 [P] [US5] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_anthropic_stream_to_chat_sequence`：验证 Anthropic 流式请求路由到 Chat 后端时，事件序列为 message_start → content_block_start → text_delta → content_block_stop → message_delta → message_stop
- [ ] T038 [P] [US5] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_stream_usage_info_position`：验证 usage token 信息在正确位置出现（Chat: 最后一个 chunk，Anthropic: message_delta + message_stop）

**Checkpoint**: 流式事件序列测试完成，US5 可独立验证

---

## Phase 7: User Story 6 - SSE 格式合规测试 (Priority: P3)

**Goal**: 验证 SSE 输出格式严格符合 SSE 规范（data: 前缀、[DONE] 终止、空行分隔）

**Independent Test**: `cargo test --test streaming_tests sse` 验证格式

### Tests for User Story 6

- [ ] T039 [P] [US6] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_sse_data_prefix`：逐事件验证每个 SSE 事件以 `data:` 前缀开头
- [ ] T040 [P] [US6] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_sse_done_termination`：验证流以 `data: [DONE]` 事件结束，之后无额外数据
- [ ] T041 [P] [US6] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_sse_empty_line_separator`：验证事件间有空行分隔，空行不包含额外空白字符
- [ ] T042 [P] [US6] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_sse_json_newlines_not_split`：验证 JSON 内容内含 `\n` 时不被错误分割为多个 SSE 事件
- [ ] T043 [P] [US6] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_sse_empty_delta_content`：验证流式事件中 delta content 为空字符串时正确处理，不进入死循环

**Checkpoint**: SSE 格式合规测试完成，US6 可独立验证

---

## Phase 8: User Story 7 - 协议特色功能 IR 转换测试 (Priority: P2)

**Goal**: 验证各协议特有高级功能（thinking、response_format、refusal、thought content blocks）经过 IR 转换后的语义完整性

**Independent Test**: `cargo test --test e2e_test feature` 验证协议特色功能

### Tests for User Story 7

- [x] T044 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_thinking_config_anthropic_roundtrip`：验证 Anthropic thinking 请求（type=consumed, budget_tokens=2048）→ decode → IR → encode → 请求中 thinking 字段完整保留
- [x] T045 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_thinking_cross_protocol_no_panic`：验证 Anthropic thinking 请求路由到 OpenAI Chat 后端时不 panic，安全降级
- [x] T046 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_thinking_content_block_roundtrip`：构造含 Thinking ContentBlock（thinking 文本 + signature）的 IrResponse，验证 Anthropic codec encode → decode 后 content type 和字段完整
- [x] T047 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_redacted_thinking_roundtrip`：构造含 RedactedThinking ContentBlock（data 字段）的 IrResponse，验证 Anthropic codec encode → decode 后字段完整
- [x] T048 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_response_format_roundtrip`：构造含 `response_format: {type: json_object}` 的 OpenAI Chat 请求，验证 decode → IR → encode 同协议往返后 response_format 字段保留
- [x] T049 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_response_format_cross_protocol_no_panic`：验证 OpenAI response_format 请求路由到 Anthropic 后端时不 panic
- [x] T050 [P] [US7] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_refusal_block_roundtrip`：构造含 Refusal ContentBlock 的 IrResponse，验证同协议 codec encode → decode 后 refusal 文本字段保留
- [x] T051 [P] [US7] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_thinking_delta_stream_event`：构造含 delta(content_type=Thinking, thinking_text="analyzing...", signature="sig_xxx") 的 IrStreamEvent，验证 Anthropic codec encode_stream_event 输出 `{"type":"thinking_delta"...}` 格式

**Checkpoint**: 协议特色功能 IR 转换测试完成，US7 可独立验证

---

## Phase 9: 收尾与回归验证 (Polish)

**Purpose**: 验证全部测试通过，文档更新，代码质量检查

- [ ] T052 [P] 在 `crates/llm-mux-gateway/tests/streaming_tests.rs` 添加 `test_stream_interruption`：验证后端流中断时返回 SSE error 事件（使用连接超时模拟）
- [ ] T053 [P] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_env_var_not_set`：验证环境变量 `${VAR}` 未设置时 config 解析为 SQL 空字符串并触发 api_key 验证错误
- [ ] T054 [P] 在 `crates/llm-mux-gateway/tests/e2e_test.rs` 添加 `test_unicode_surrogate_pairs`：验证响应中包含 emoji 等 surrogate pair 时 UTF-8 编解码不发生截断
- [ ] T055 运行 `cargo test --test e2e_test --test streaming_tests` 确保全部测试通过
- [ ] T056 运行 `cargo clippy --all-targets` 确保无警告
- [ ] T057 运行 `cargo fmt --all -- --check` 确保格式化一致

**Checkpoint**: 全部测试通过，功能交付就绪

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: 无依赖 — 必须最先完成
- **Phase 2-8 (US1-US7)**: 依赖 Phase 1 完成 — 各故事之间无相互依赖
- **Phase 9 (Polish)**: 依赖所有用户故事完成

### User Story Dependencies

- **US1 (边界条件)**: Phase 1 后可立即开始 — 独立于其他故事
- **US2 (协议转换)**: Phase 1 后可立即开始 — 独立于 US1
- **US3 (模型路由)**: Phase 1 后可立即开始 — 独立于 US1/US2，但需复用 US2 中的部分配置模式
- **US4 (异常容错)**: Phase 1 后可立即开始 — 独立于其他故事
- **US5 (流式序列)**: Phase 1 后可立即开始 — 依赖 streaming_tests.rs 创建（T005）
- **US6 (SSE 格式)**: Phase 1 后可立即开始 — 依赖 streaming_tests.rs 创建（T005）
- **US7 (协议特色功能)**: Phase 1 后可立即开始 — 验证 codec 层编解码完整性，部分测试不依赖网络

### Within Each User Story

- 每个故事内的 [P] 测试函数可并行编写
- 函数间无依赖（各自独立启动服务器）

### Parallel Opportunities

- Phase 1: T003/T004 可并行
- US1: T007-T017 全部 11 个任务可并行（不同测试函数）
- US2: T018-T024 全部 7 个任务可并行
- US3: T025-T029 全部 5 个任务可并行
- US4: T030-T035 全部 6 个任务可并行
- US5: T036-T038 全部 3 个任务可并行
- US6: T039-T043 全部 5 个任务可并行
- US7: T044-T051 全部 8 个任务可并行
- Polish: T052/T053/T054 可并行
- **跨故事**: US1-US7 七个故事彼此独立，可完全并行开发

---

## Parallel Example: User Story 1

```bash
# 并行启动 US1 的全部测试任务:
Task: "T007 test_empty_request_body"
Task: "T008 test_whitespace_only_body"
Task: "T009 test_missing_model_field"
Task: "T010 test_empty_model_name"
Task: "T011 test_empty_messages_array"
Task: "T012 test_unicode_special_chars"
Task: "T013 test_injection_like_prompt"
Task: "T014 test_large_payload"
Task: "T015 test_unknown_top_level_fields"
Task: "T016 test_concurrent_10_requests"
Task: "T017 test_concurrent_different_models"
```

---

## Parallel Example: Streaming Tests (US5 + US6)

```bash
# 并行启动全部流式测试任务:
Task: "T036 test_chat_stream_to_anthropic_sequence"
Task: "T037 test_anthropic_stream_to_chat_sequence"
Task: "T038 test_stream_usage_info_position"
Task: "T039 test_sse_data_prefix"
Task: "T040 test_sse_done_termination"
Task: "T041 test_sse_empty_line_separator"
Task: "T042 test_sse_json_newlines_not_split"
Task: "T043 test_sse_empty_delta_content"
```

---

## Implementation Strategy

### MVP First (US1 + US2)

1. Complete Phase 1: Setup (测试框架增强)
2. Complete Phase 2: US1 (边界条件测试) — 最常见故障场景覆盖
3. Complete Phase 3: US2 (协议转换完整性) — 网关核心功能验证
4. **STOP and VALIDATE**: 对核心功能建立信心
5. 两个 P1 故事完成即 MVP

### Incremental Delivery

1. Phase 1 (Setup) → 框架就绪
2. Phase 2 (US1) → 边界条件覆盖
3. Phase 3 (US2) → 核心协议转换验证 (MVP!)
4. Phase 4 (US3) → 路由匹配验证
5. Phase 5 (US4) → 异常容错验证
6. Phase 6 (US5) → 流式序列验证
7. Phase 7 (US6) → SSE 格式合规
8. Phase 8 (US7) → 协议特色功能 IR 转换验证
9. Phase 9 (Polish) → 回归 + 质量检查

### Parallel Team Strategy

- Developer A: US1 (边界条件) + US4 (异常容错)
- Developer B: US2 (协议转换) + US3 (路由)
- Developer C: US5 (流式序列) + US6 (SSE 格式)
- Developer D: US7 (协议特色功能 IR 转换)

---

## Notes

- [P] 任务 = 不同测试函数，无共享状态，可并行
- [Story] 标签将任务映射到 spec.md 中的用户故事
- 每个用户故事可独立测试：运行对应测试子集验证
- 除 T005（创建 streaming_tests.rs）外，Phase 2-8 的所有任务无顺序依赖
- 边界条件测试（15 个场景）和协议转换测试（7 个场景）为最高优先级
- US7 的 codec 层测试（T044, T046-T050）不依赖网络，可离线运行
- T030-T035 异常测试不依赖真实后端 API（使用模拟地址），可在无网络环境运行
- 每次完成一组 [P] 任务后运行 `cargo test --test <file>` 验证

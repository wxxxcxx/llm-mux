# Feature Specification: 复杂场景集成测试

**Feature Branch**: `002-complex-integration-tests`

**Created**: 2026-05-27

**Status**: Draft

**Input**: User description: "添加包含复杂场景集成测试，考虑各种边界条件"

## Clarifications

### Session 2026-05-27

- Q: 运行 `cargo test` 时网络依赖测试是否默认执行？ → A: 使用 Cargo feature `integration` 控制，不加 feature 只跑单元测试

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 边界条件集成测试 (Priority: P1)

作为开发者，我需要验证网关在各种边界条件下的行为是否正确，包括空内容、超长输入、特殊字符、并发请求等场景，确保网关在极端情况下不会崩溃或产生数据错误。

**Why this priority**: 边界条件是生产环境中最常见的故障源，必须先覆盖以建立对系统稳定性的信心。

**Independent Test**: 运行边界条件测试套件，验证每个边界场景返回预期结果或合理的错误信息。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 发送空消息体的请求，**Then** 返回 400 错误，不崩溃
2. **Given** 网关正在运行，**When** 发送超长消息（> 100KB）的请求，**Then** 系统正确处理或返回明确的大小限制错误
3. **Given** 网关正在运行，**When** 发送包含 Unicode 特殊字符（emoji、中文、阿拉伯文）的请求，**Then** 响应中的内容完整保留原始字符
4. **Given** 网关正在运行，**When** 发送包含 HTML 标签、SQL 片段等潜在注入内容的提示词，**Then** 网关透传内容不做过滤（不改变语义）
5. **Given** 网关正在运行，**When** 同时发送 10 个并发请求，**Then** 所有请求都能正常完成，无数据串扰
6. **Given** 网关正在运行，**When** 后端返回 HTTP 500 错误，**Then** 网关以客户端协议格式返回可理解的错误信息，而非原始 dump

---

### User Story 2 - 协议转换完整性测试 (Priority: P1)

作为开发者，我需要验证跨协议转换时所有关键字段的完整性和正确性，包括系统提示词的多段结构、携带图片的多模态请求、包含工具调用历史的对话、各种 stop_reason 的映射等。

**Why this priority**: 协议转换是网关的核心价值，字段丢失或映射错误会直接影响模型调用质量。

**Independent Test**: 对每种跨协议场景发送请求并检查返回的响应字段是否完整且正确映射。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 发送包含 system prompt（多段文本块）的 Anthropic Messages 请求到 OpenAI Chat 路由，**Then** 后端收到正确合并的多段 system 消息
2. **Given** 网关正运行，**When** 发送包含图片 URL 的多模态请求，**Then** 跨协议转换后图片 URL 和媒体类型信息完整保留
3. **Given** 网关正运行，**When** 发送包含多轮工具调用历史（assistant tool_use / tool result）的请求，**Then** 多轮工具调用历史在协议转换后完整保留
4. **Given** 网关正运行，**When** 后端返回 stop_reason 为 tool_use 的响应，**Then** 转换为 OpenAI Chat 格式时 finish_reason 正确映射为 tool_calls
5. **Given** 网关正运行，**When** 后端返回 stop_reason 为 content_filter 的响应，**Then** 转换为 OpenAI Chat 格式时 finish_reason 正确映射为 content_filter
6. **Given** 网关正运行，**When** 后端返回 stop_reason 为 end_turn 的响应，**Then** 转换为 OpenAI Chat 格式时 finish_reason 正确映射为 stop

---

### User Story 3 - 模型路由与匹配测试 (Priority: P2)

作为开发者，我需要验证路由规则在所有条件下都能正确匹配，包括通配符、精确匹配、多条件组合、fallback 兜底，以及模型映射功能。

**Why this priority**: 路由是请求正确分发的基础，错误的路由会导致请求发送到错误的模型。

**Independent Test**: 对每种路由配置发送请求，验证路由结果是否正确。

**Acceptance Scenarios**:

1. **Given** 配置了 `claude-*` 通配符路由，**When** 发送 model="claude-sonnet-4-6" 的请求，**Then** 正确匹配到 Anthropic 路由
2. **Given** 配置了精确匹配路由 `gpt-4o`，**When** 发送 model="gpt-4o" 的请求，**Then** 精确匹配优先于通配符匹配
3. **Given** 配置了 model_mapping 映射 `gpt-4o → claude-sonnet-4-6`，**When** 发送 model="gpt-4o" 的请求，**Then** 后端实际收到的 model 为 `claude-sonnet-4-6`，但返回时保持为 `gpt-4o`
4. **Given** 配置了多级路由规则，**When** 发送的请求不匹配前几级规则，**Then** 最终命中 `*` fallback 规则而非返回错误

---

### User Story 4 - 异常恢复与容错测试 (Priority: P2)

作为开发者，我需要验证网关在异常情况下的恢复能力和容错行为，包括后端不可用、网络超时、流中断、无效响应等场景。

**Why this priority**: 网关作为中间层必须有合理的错误处理机制，不能因为后端异常导致自身崩溃或内存泄漏。

**Independent Test**: 模拟各种后端故障场景，验证网关的容错行为。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 后端连接超时（无响应 30 秒），**Then** 网关返回 502 错误并正确释放资源
2. **Given** 网关正运行，**When** 后端返回非 JSON 格式的响应体，**Then** 网关返回 502 错误，不崩溃
3. **Given** 网关正运行且流式请求在进行中，**When** 后端流中断（连接复位），**Then** 网关返回 SSE 格式的 error 事件并包含 [DONE]
4. **Given** 网关正运行，**When** 后端返回的 JSON 响应缺少必填字段（如 choices），**Then** 网关返回 502 错误并提供有意义的错误信息
5. **Given** 网关正运行，**When** 同时发起 50 个请求并逐一取消其中 25 个，**Then** 网关正确清理被取消请求的资源，不影响其余 25 个请求
6. **Given** 网关接收到无效 JSON 格式的请求体，**When** 协议解析失败，**Then** 网关返回 400 错误，不崩溃

---

### User Story 5 - 流式事件序列完整性测试 (Priority: P3)

作为开发者，我需要验证流式传输的事件序列完整性和顺序正确性，包括 content_block_start → delta → content_block_stop → message_stop 的完整生命周期，以及跨协议转换后事件类型的正确映射。

**Why this priority**: 流式传输是 LLM API 的重要特性，事件序列错误会导致客户端解析异常。

**Independent Test**: 对每种流式协议组合发送 stream: true 请求，验证事件序列和类型映射。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 发送流式 Chat 请求路由到 Anthropic 后端，**Then** 流事件序列包含: chat.completion.chunk(role: assistant) → delta(content) → delta(tool_calls)[可选] → finish_reason : stop → [DONE]
2. **Given** 网关正运行，**When** 发送流式 Anthropic 请求路由到 Chat 后端，**Then** 流事件序列包含: message_start → content_block_start(text) → text_delta → content_block_stop → message_delta(stop_reason) → message_stop
3. **Given** 网关正运行，**When** 流式传输中收到 usage 信息，**Then** usage 信息出现在正确的事件位置（Chat 在最后一个 chunk，Anthropic 在 message_delta 和 message_stop）

---

### User Story 6 - SSE 格式合规测试 (Priority: P3)

作为开发者，我需要验证网关的 SSE 输出格式严格符合 SSE 规范，确保客户端 SDK 能正确解析。

**Why this priority**: SSE 格式错误会导致客户端解析失败，表现为静默错误难以排查。

**Independent Test**: 对 SSE 输出流逐字节验证格式合规性。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 流式请求完成，**Then** 流以 `data: [DONE]` 事件结束
2. **Given** 网关正运行，**When** 发出流式请求，**Then** 每个事件均以 `data:` 前缀开头，后跟 JSON 内容
3. **Given** 网关正运行，**When** 流式传输中出现空行，**Then** 空行不包含额外空格或不可见字符
4. **Given** 网关正运行，**When** 流式传输中包含多行数据（JSON 内含 `\n`），**Then** 数据不被错误地分割成多个 SSE 事件

---

### User Story 7 - 协议特色功能 IR 转换测试 (Priority: P2)

作为开发者，我需要验证各协议特有的高级功能（thinking 扩展推理、response_format 结构化输出、refusal 拒绝、thought content blocks）在经过 IR 中间层转换后是否保持语义正确，确保协议特色功能不会在转换过程中被静默丢弃。

**Why this priority**: 协议特色功能是不同 LLM 提供商的差异化竞争力。如果 thinking 内容、结构化输出等高级功能在协议转换中被丢弃，用户将无法充分利用各模型的能力。

**Independent Test**: 对每种协议特色功能构造含该功能的请求，经过编解码往返（decode → IR → encode）后验证 IR 层面字段完整性，并验证同协议往返后的协议层面字段完整性。

**Acceptance Scenarios**:

1. **Given** 网关正运行，**When** 发送含 Anthropic thinking 配置的 Messages 请求（`thinking: {type: enabled, budget_tokens: 2048}`），**Then** IR 中 `thinking.mode = "enabled"` 且 `thinking.budget_tokens = 2048` 在 decode → encode 同协议往返后完整保留
2. **Given** 网关正运行，**When** 发送含 Anthropic thinking 配置的请求且路由到 OpenAI Chat 后端，**Then** thinking 配置被记录在 IR 中（而非 panic），网关正常处理请求
3. **Given** 网关正运行，**When** Anthropic 后端返回含 Thinking ContentBlock 的响应，**Then** IR 中 thinking content block（含 thinking 文本和 signature 签名字段）在同协议 encode 后完整保留
4. **Given** 网关正运行，**When** Anthropic 后端返回含 RedactedThinking ContentBlock 的响应，**Then** IR 中 redacted_thinking content block 在同协议 encode 后完整保留
5. **Given** 网关正运行，**When** OpenAI Chat 请求包含 `response_format: {type: json_object}`，**Then** IR 中 `response_format.kind = "json_object"` 在 decode → encode 同协议往返后完整保留
6. **Given** 网关正运行，**When** OpenAI Responses 后端返回含 Refusal ContentBlock 的响应，**Then** IR 中 refusal content block 在同协议 encode 后完整保留
7. **Given** 网关正运行，**When** 流式请求中 Anthropic 后端返回 `thinking_delta` 和 `signature_delta` 事件，**Then** IR 流事件中 delta 的 content_type 正确识别为 Thinking，同协议 encode 后事件类型恢复为 `thinking_delta`

---

### Edge Cases

- 请求 body 为空或全空白字符时，返回 400 并提示"空请求体"
- 请求 body 为超过 10MB 的巨大 payload 时，网关拒绝并返回 413
- 请求中 model 字段为空字符串时，应返回明确的模型验证错误
- 请求中 model 字段缺失时，应返回模型验证错误（非 panic）
- 请求中 messages 数组为空时，网关识别并返回验证错误
- 请求中包含未知的顶层 JSON 字段时，字段应被透传到 provider_extensions
- 后端返回 HTTP 429（限流）时，网关透传限流错误信息
- 后端返回 HTTP 503（服务不可用）时，网关返回 502 并包含原始错误上下文
- config.yaml 中 provider 引用不存在的 provider 名称时，config validate 应报错
- config.yaml 中 routes 数组为空时，config validate 应报错
- 环境变量 `${VAR}` 未设置时，config 解析为 SQL 空字符串并触发 api_key 验证错误
- 同时发送 50 个带不同 model 名称的并发请求时，各自独立路由，无串扰
- 请求的 model 匹配到多个路由规则时，按配置顺序从上到下优先匹配
- 后端响应包含 unicode 代理对（如 emoji）时，UTF-8 编解码不发生截断
- 流式事件中 delta 的 content 为空字符串时，客户端正常处理且不进入死循环
- Anthropic thinking 请求经跨协议（OpenAI Chat）路由时，thinking 配置不会被 panic（应安全降级）
- response_format 经跨协议（Anthropic）路由时，response_format 字段被安全丢弃（不应 panic）
- Thinking ContentBlock 在响应中但目标协议不支持时，不应导致整个响应编码失败
- Anthropic 流式事件的 thinking_delta 在跨协议转换中不被识别为非法事件类型

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 系统必须在集成测试中覆盖至少 10 种边界条件场景，包括空请求、超长输入、特殊字符、并发请求
- **FR-002**: 系统必须在集成测试中验证两种跨协议路由方向（OpenAI Chat → Anthropic Messages 和 Anthropic Messages → OpenAI Chat）的字段完整性
- **FR-003**: 系统必须在集成测试中验证 system prompt 的多段文本块在协议转换中正确保留
- **FR-004**: 系统必须在集成测试中验证图片 URL 和文档内容在多模态请求中跨协议保留
- **FR-005**: 系统必须在集成测试中验证多轮工具调用历史（tool_use、tool_result）在协议转换中的完整性
- **FR-006**: 系统必须在集成测试中验证所有 stop_reason 值到 finish_reason 的正确映射
- **FR-007**: 系统必须在集成测试中验证路由规则的上到下匹配、通配符、精确匹配、fallback 规则
- **FR-008**: 系统必须在集成测试中验证 model_mapping 功能正确替换模型名并通过 upstream_url 透传
- **FR-009**: 系统必须在集成测试中验证后端返回 HTTP 5xx、4xx、非 JSON 响应时的网关容错行为
- **FR-010**: 系统必须在集成测试中验证流式请求的后端中断场景下 SSE error 事件的格式
- **FR-011**: 系统必须在集成测试中验证 SSE 输出的事件格式合规性（data: 前缀、[DONE] 终止、空行分隔）
- **FR-012**: 系统必须在集成测试中验证 usage token 信息在流式响应中的正确位置和值
- **FR-013**: 系统必须为每个测试失败输出清晰的断言消息，包含预期值和实际值，以便快速定位问题
- **FR-014**: 系统必须在集成测试中验证 Anthropic thinking 扩展推理功能（thinking type、budget_tokens、signature）经过 IR 后在同协议往返中完整保留
- **FR-015**: 系统必须在集成测试中验证 Thinking、RedactedThinking、Refusal 等特殊 ContentBlock 在 decode → IR → encode 同协议往返中不丢失
- **FR-016**: 系统必须在集成测试中验证 OpenAI response_format 结构化输出字段经过 IR 后在同协议往返中完整保留
- **FR-017**: 依赖网络的集成测试必须通过 Cargo feature `integration` 控制，`cargo test` 不加 feature 时仅运行不依赖网络的测试

### Key Entities

- **集成测试用例**: 包含输入请求、路由配置、预期输出的完整定义
- **测试配置**: 动态生成的 YAML 配置，为每个测试场景定制路由规则和 provider 设置
- **验证断言**: 对响应状态码、响应体结构、关键字段值、事件序列的顺序和内容的校验

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 边界条件测试覆盖不少于 15 个不同场景，每个场景有明确的预期结果
- **SC-002**: 两种协议转换方向（OpenAI Chat ↔ Anthropic Messages）各不少于 4 个测试用例验证字段完整性（含协议特色功能）
- **SC-003**: 路由匹配测试覆盖通配符、精确匹配、多条件路由、model_mapping、fallback 共 5 类场景
- **SC-004**: 异常容错测试覆盖 4xx、5xx、连接超时、非 JSON 响应、流中断共 5 类异常
- **SC-005**: 所有集成测试在 60 秒内完成单次运行（不含网络延迟）
- **SC-006**: 每个测试失败时，断言消息包含足够的上下文使开发者能在 2 分钟内定位问题

## Assumptions

- 集成测试使用真实的后端 API（opencode.go），但会在测试中设置合理的超时时间
- 测试配置（provider url、api_key）从项目的 `.env` 文件加载
- 并发测试场景中使用确定性请求以保证结果可重复
- 测试运行在允许外部网络连接的环境中
- 边界条件的定义参考项目中已有的 `plan.md` 和 `spec.md` 中描述的性能和功能约束
- 测试框架使用 Rust 的 `tokio::test` 运行时，每个测试独立启动服务器实例
- 网络依赖测试通过 Cargo feature `integration` 门控：`cargo test --features integration` 运行完整套件，`cargo test` 仅运行不需要网络的测试

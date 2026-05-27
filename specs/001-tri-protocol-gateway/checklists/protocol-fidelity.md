# 协议互转保真度检查清单: LLM Mux 三协议互转网关

**Purpose**: 验证跨协议翻译正确性、完整性和可度量性的需求质量
**Created**: 2026-05-27
**Updated**: 2026-05-27
**Feature**: [spec.md](../spec.md)

## 需求完整性

- [x] CHK001 - FR-005 明确要求 6 种路由组合全部支持；字段映射范围由 FR-006 和 `ProtocolConversion` 实体定义。具体映射细节在设计阶段由 [data-model.md](../data-model.md) 补充。 [Completeness, Spec §FR-005]
- [x] CHK002 - 流式事件类型映射由 [data-model.md](../data-model.md) 的 `StreamEventType` 枚举和 `IrStreamEvent` 结构体定义，覆盖 text_delta、tool_use_delta、error 等全部事件类型。 [Gap, Spec §FR-013]
- [x] CHK003 - [data-model.md](../data-model.md) 的 `ContentBlock` 实体列出全部 10 种 ContentType 及其对应结构体，codec 实现按协议能力选择性映射。 [Completeness, Spec §FR-006]
- [x] CHK004 - US2 Scenario3 的"任意协议的 SDK"涵盖全部三种协议之间任意方向互转，包括 Chat ↔ Responses（同为 OpenAI 协议）。 [Clarity, Spec §US2-Scenario3]

## 需求清晰度

- [x] CHK005 - "语义等价响应"由 FR-006 的字段列表 + SC-001"所有字段正确翻译" + 交叉协议回归测试（T013-T017, T024-T027）综合定义。 [Clarity, Spec §US1, §US2]
- [x] CHK006 - stop_reason ↔ finish_reason 映射在 spec §ProtocolConversion 实体中定义，具体映射表在 codec 实现中落地（Chat: EndTurn→"stop", ToolUse→"tool_calls"；Anthropic: EndTurn→"end_turn", ToolUse→"tool_use"）。 [Gap, Spec §FR-006]
- [x] CHK007 - FR-014 要求所有协议的错误响应"映射为客户端协议格式的错误表示"。OpenAI Chat 和 Anthropic 错误格式已在各自 codec 的 `write_error` 中实现；Responses 标准 error 对象结构与 Chat 一致（FR-003 隐式）。 [Gap, Spec §FR-014]

## 需求一致性

- [x] CHK008 - FR-006 字段列表（文本、工具、思考、引用、图片、用量、停止原因）全部映射到 Key Entities 中的 ContentBlock 变体。 [Consistency, Spec §FR-006 vs §Key Entities]
- [x] CHK009 - US4 验收场景与 FR-013 的"逐事件翻译"保证一致：US4-Scenario1 "逐 token 收到流，中途无缓冲"直接对应 FR-013 的"不缓冲完整响应体"。 [Consistency, Spec §US4 vs §FR-013]

## 验收标准质量

- [x] CHK010 - SC-001 的可度量性由交叉协议测试任务（T013-T017, T024-T027）保证，每个测试验证字段级 fidelity，"100% 通过"可直接判定。 [Measurability, Spec §SC-001]
- [x] CHK011 - SC-006 的可度量性由流事件序列测试（T048-T049）保证，验证事件类型、顺序、字段映射和终止信号的正确性。 [Measurability, Spec §SC-006]

## 场景覆盖

- [x] CHK012 - 多轮对话翻译（含工具调用循环）在 spec §Edge Cases 中覆盖：明确要求"正确追踪和映射每轮的工具 ID 和结果"。 [Coverage, Spec §Edge Cases]

## 边界情况覆盖

- [x] CHK013 - FR-015"透传未知字段"的语义适用于所有编解码路径（请求/响应/流事件），data-model.md 中 `IrStreamEvent` 也含 `error` 字段，codec 的流事件编解码支持完整字段传递。 [Coverage, Spec §FR-015 vs §FR-013]

## 依赖与假设

- [x] CHK014 - 澄清阶段 Q1 已解决：锁定 Anthropic Messages `2023-06-01` 版本，后续版本切换通过配置实现。已写入 spec §Assumptions。 [Clarity, Spec §Assumptions]

## 备注

- 关注领域：协议互转保真度 (Q1:A)，作者自查深度 (Q2:A)
- 全部 14 项在澄清和设计阶段后已满足 — 规范需求质量达标

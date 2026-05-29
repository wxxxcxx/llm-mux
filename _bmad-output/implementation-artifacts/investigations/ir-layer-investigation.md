# Investigation: IR 层实现验证

## Hand-off Brief

1. **调查目标：** 验证自定义 IR 中间层（IrRequest/IrResponse/ContentBlock 等）是否正确实现，以及它与 Codec/Adapter 两层 trait 的关系是否一致
2. **当前状态：** IR 类型定义完整，但存在两处架构不一致和一处设计遗漏
3. **下一步：** 审查发现后决定是否修复

## Case Info

| Field | Value |
|-------|-------|
| Ticket | ir-layer-verification |
| Date opened | 2026-05-29 |
| Status | Active |
| Evidence sources | `crates/core/src/{ir.rs, types.rs, codec.rs, adapter.rs}`, `crates/adapters/*/`, `crates/gateway/src/handlers/mod.rs` |

## Problem Statement

验证 IR 中间层的设计目标——"adapter 不依赖 genai，流式和非流式路径统一使用自定义 IR"——是否在代码中一致实现。

## Confirmed Findings

### Finding 1: IR 类型定义完整 (✅ Confirmed)

`crates/core/src/ir.rs` 和 `types.rs` 提供了完整的自定义 IR 类型体系：

| 类型 | 说明 |
|------|------|
| `IrRequest` | 统一请求：model, messages, system_prompt, tools, tool_choice, 采样参数, thinking, response_format, provider_extensions |
| `IrResponse` | 统一响应：id, model, content, stop_reason, usage, provider_extensions |
| `IrStreamEvent` | 流事件：event_type, delta, stop_reason, usage, error |
| `ContentBlock` | 内容块判别联合：text/image/tool_use/tool_result/thinking/redacted_thinking/refusal 等 10 种类型 |
| `IrTool` / `IrToolChoice` | 工具定义与选择 |
| `IrUsage` | Token 统计（含 cache/thinking 字段） |

**证据：** `crates/core/src/ir.rs:1-263`, `crates/core/src/types.rs:1-203`

### Finding 2: 非流式路径正确使用自定义 IR (✅ Confirmed)

非流式请求路径：`HTTP → Codec::decode_request() → IrRequest → (gateway) → IrResponse → Codec::encode_response() → HTTP`

- `Codec` trait 完全使用自定义 IR 类型：`codec.rs:27-39`
- 三种适配器均实现 `Codec` trait，不直接依赖 genai
- 测试充分：`cross_protocol_tests.rs:8-64` 验证 decode/encode roundtrip

**证据：** `crates/core/src/codec.rs:27-39`, `crates/adapters/openai/src/chat/mod.rs:22-165`

### Finding 3: 流式路径绕过 Codec trait，使用 genai 类型 (⚠️ 架构不一致)

流式路径：`HTTP → Codec::decode_request() → IrRequest → (gateway) → genai ChatStream → Adapter::encode_stream_event(genai events) → SSE`

关键区别：
- 流式事件的编码使用 `Adapter` trait（`crates/core/src/adapter.rs:30`），它接受 genai `ChatStreamEvent` 类型
- `handlers/mod.rs:434` 使用 `Box<dyn Adapter>` 而非 `Box<dyn Codec>` 处理流式
- `IrStreamEvent` 类型在自定义 IR 中定义，但流式路径完全未使用它

**证据：**
- `crates/core/src/adapter.rs:30` — `fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError>`
- `crates/gateway/src/handlers/mod.rs:434` — `let adapter: Box<dyn llm_mux_core::adapter::Adapter>`
- `crates/gateway/src/handlers/mod.rs:448` — `adapter.encode_stream_event(&evt)`

### Finding 4: IrStreamEvent 是死代码 (⚠️ 设计遗漏)

`IrStreamEvent` 结构体完整定义，在 `lib.rs` 中重导出，在 `Converter` trait 的 `convert_stream_event` 方法中被引用，但：

1. `Codec` trait **没有任何流相关方法**
2. `Converter` trait 的 `convert_stream_event` 仅由 `NoopConverter` 实现（空操作）
3. 没有实际的 `Converter` 实现被使用

`IrStreamEvent` 目前仅存在于类型系统中，没有实际的转换路径使用它。

**证据：**
- IrStreamEvent 定义: `ir.rs:180-196`
- Codec trait 无流方法: `codec.rs:27-39`
- NoopConverter: `codec.rs:140-160`
- 唯一使用 dyn Adapter: `handlers/mod.rs:434`

### Finding 5: ir.rs 中的 genai 依赖 (⚠️ 轻度不一致)

`crates/core/src/ir.rs:254-262` 中的 `stop_reason_from_genai()` 函数直接引用 `genai::chat::StopReason`，使 `core` crate 的 `ir` 模块产生了对 genai 的依赖。

虽然 `core/Cargo.toml` 已经依赖 genai（用于 `adapter.rs`），但这个函数的放置位置在语义上属于 gateway 层的桥接逻辑，放在 `ir.rs` 中不太合理。

**证据：** `crates/core/src/ir.rs:254-262`

### Finding 6: ContentBlock 缺少编译期类型安全 (ℹ️ 设计取舍)

`ContentBlock` 使用 Optional 字段 + `ContentType` 判别器的模式，导致：

```rust
// 编译通过，但语义错误：
ContentBlock {
    content_type: ContentType::Text,
    tool_use: Some(...),      // 不会报错
    ..Default::default()
}
```

这是 Rust 中实现判别联合的标准模式，没有运行时开销但缺少编译期保证。三个适配器的实现均正确配对 `content_type` 和对应字段。

**证据：** `crates/core/src/types.rs:177-203`

## Conclusion

**Confidence: High**

### IR 层设计评估

| 方面 | 状态 | 说明 |
|------|------|------|
| IR 类型完整性 | ✅ 完整 | 覆盖 LLM API 全部常见参数 |
| 非流式 Codec 一致性 | ✅ 一致 | Codec ↔ IrRequest/IrResponse 闭环 |
| 流式路径一致性 | ⚠️ 不一致 | 流式使用 genai 类型而非 IrStreamEvent |
| IrStreamEvent 可用性 | ❌ 死代码 | 定义但无实际使用路径 |
| genai 依赖隔离 | ⚠️ 轻度 | `ir.rs` 中 `stop_reason_from_genai` |
| ContentBlock 类型安全 | ℹ️ 可接受 | Optional 字段模式，各 adapter 使用正确 |

### 最核心的问题

流式路径与非流式路径使用了**两套不同的 trait**：
- 非流式: `Codec` trait → 自定义 IR ✅
- 流式: `Adapter` trait → genai 类型 ❌

这意味着如果要切换 AI Client，非流式路径只需改 gateway 层映射逻辑，但流式路径需要同时修改 adapter 中的 `encode_stream_event` 实现，因为它直接依赖 genai `ChatStreamEvent` 类型。这与"自建 IR 防止依赖 genai"的设计目标部分矛盾。

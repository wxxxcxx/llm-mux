# Feature Specification: 基于 genai 适配器模式重构协议网关

**Feature Branch**: `003-genai-adapter-refactor`

**Created**: 2026-05-28

**Status**: Draft

**Input**: User description: "我想基于genai重构我的项目，我发现genai是对于不同的平台做的单独适配，这个项目也可以采用这个思路，IR应该定义为genai方便使用的结构，工作中心集中于将不同的api请求翻译为IR，以及将IR响应翻译回请求对应的API响应格式"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 定义 genai 兼容的统一 IR (Priority: P1)

开发者希望 LLM Mux 的内部表示（IR）与 genai 库的类型体系对齐——使用 genai 的 `ChatRequest`、`ChatResponse`、`MessageContent`、`ContentPart`、`Tool`、`ToolCall`、`ToolResponse` 等核心类型作为协议无关的内部中间层。这消除了现有自定义 IR 类型与行业标准库之间的概念映射开销，让每个协议适配器只需专注于"外部协议格式 ↔ genai IR"的单向翻译，而不需要再维护一套自有的抽象。

**Why this priority**: IR 是整个网关的数据中枢。以 genai 类型作为 IR 意味着适配器逻辑可以直接复用 genai 的类型定义和推理模型，避免重复造轮子。这是重构的基石，必须先于适配器拆分完成。

**Independent Test**: 用 genai 的 `ChatRequest` 构造一个完整请求（含文本消息、图片、工具定义、采样参数），验证所有字段可被后续适配器正确消费。

**Acceptance Scenarios**:

1. **Given** IR 定义为 genai 类型体系（即 `ChatRequest`、`ChatResponse`、`ChatStreamEvent` 等），**When** 查看 IR 数据模型文档，**Then** 文档中的类型名称、字段语义与 genai 官方 API 文档一致，不存在自有的自定义消息/工具/响应抽象。
2. **Given** IR 使用 genai `MessageContent` + `ContentPart` 作为内容载体，**When** 构造含文本、图片 (base64/URL)、工具调用、工具结果的混合内容请求，**Then** 所有内容形式均可在 IR 中正确表示和传递。
3. **Given** IR 使用 genai 的 `ChatOptions` 作为请求参数载体（temperature、max_tokens、top_p、stop_sequences、reasoning_effort 等），**When** 协议适配器从外部请求中提取参数写入 IR，**Then** genai 支持的所有通用参数均可在 IR 中表示；genai 不支持但协议独有的参数进入 `extra_body` / 扩展字段。

---

### User Story 2 - 协议适配器独立拆分 (Priority: P2)

为三种外部协议（OpenAI Chat Completions、OpenAI Responses、Anthropic Messages）各实现独立的适配器，每个适配器的职责为：
- **入站 (decode)**：将外部协议的 HTTP 请求体解析为该协议的领域模型（已是 genai 兼容结构），然后归一化为 IR
- **出站 (encode)**：将 IR 响应/流事件编码为外部协议格式的 HTTP 响应

每个适配器独立开发、独立测试、独立演进，与 genai 库自身的 `AdapterKind` 枚举（`OpenAI`, `OpenAIResp`, `Anthropic`, ...）一一对应，代码层面完全隔离，不共享非通用逻辑。

**Why this priority**: 适配器独立拆分是 genai 设计模式的核心——每个 provider 有自己的 adapter，IR 则跟随 genai 的类型定义。适配器之间互不依赖，新增或修改一个协议不影响其他协议。

**Independent Test**: 修改 Anthropic Messages 适配器的消息映射逻辑，在未修改 OpenAI Chat 适配器代码的情况下，运行 OpenAI 协议的全量测试套件应 100% 通过。

**Acceptance Scenarios**:

1. **Given** 项目中存在 `adapter/` 目录，**When** 查看目录结构，**Then** 包含三个独立子目录 `openai/`、`openai_resp/`、`anthropic/`，各自完整实现其协议的解码/编码逻辑。
2. **Given** 任一适配器模块仅依赖 `genai` 类型和自身协议的解析库（如 serde），**When** 查看其 `Cargo.toml` 的依赖，**Then** 不出现其他协议适配器的依赖或引用。
3. **Given** 适配器对外暴露统一的 trait 接口（如 `Adapter { fn decode_request(...); fn encode_response(...); fn decode_stream_event(...); fn encode_stream_event(...); }`），**When** 接入新的后端 provider（如 Gemini），**Then** 只需实现该 trait 即可接入路由系统，无需修改路由核心逻辑。

---

### User Story 3 - 集成 genai 作为下游调用层 (Priority: P3)

LLM Mux 的下游调用层直接使用 genai 的 `Client` 发起对后端 LLM 服务的请求，替代现有自维护的 HTTP 客户端和协议构造代码。路由层将匹配到的后端配置（model、endpoint、auth）转换为 genai 的 `ServiceTarget`，调用 `client.exec_chat()` / `client.exec_chat_stream()`，genai 负责构造正确的协议请求体、解析响应和流事件。

**Why this priority**: genai 已经完成了多 provider 的协议构造和流式解析工作，复用它可以删除项目中的大量样板代码，同时获得 genai 持续的 provider 兼容性更新（新模型、新参数等）。

**Independent Test**: 配置一个真实或 mock 后端，通过 genai Client 发送一个 ChatRequest，验证请求到达后端且响应被正确解析为 genai ChatResponse。

**Acceptance Scenarios**:

1. **Given** LLM Mux 路由层确定后端为 Anthropic 模型，**When** 构建 genai `ModelIden` + `AuthData` + `Endpoint` 并调用 `client.exec_chat(model, request, options)`，**Then** genai 构造正确的 Anthropic Messages API 请求体并发起 HTTP 调用，返回 `ChatResponse`。
2. **Given** 流式请求场景，**When** 调用 `client.exec_chat_stream()`，**Then** 返回的 `ChatStream` 正确迭代出 `ChatStreamEvent::Chunk` / `ReasoningChunk` / `ToolCallChunk` / `End` 事件，无需自行解析 SSE。
3. **Given** genai Client 处理了后端返回的 HTTP 错误（429、502 等），**When** 后端返回 429 限流响应，**Then** genai 将错误封装为 `genai::Error`，LLM Mux 映射为客户端协议格式的错误响应。

---

### User Story 4 - 功能回归与覆盖增强 (Priority: P4)

重构完成后，原有功能（三种协议互转、流式传输、工具调用、JSON Schema 结构化输出、Reasonable Effort 推理控制、Prompt 缓存）必须保持完全可用，且经由 genai 的类型系统，原本不支持或处理不完整的功能应当获得改善：如 `frequency_penalty`、`presence_penalty`、`top_k` 等 genai 已抽象但不完整覆盖的参数应通过 `extra_body` 透传机制保留。

**Why this priority**: 保证重构不回归是底线要求；利用 genai 的 `ChatOptions.extra_body` 低层逃逸机制可以弥补 genai 抽象层未覆盖的 provider-specific 参数，提升整体协议保真度。

**Independent Test**: 运行已有全量集成测试套件（spec 002 定义），所有测试用例通过。额外验证：发送含 `frequency_penalty` 的请求到 OpenAI Chat 后端，该参数正确透传至下游请求。

**Acceptance Scenarios**:

1. **Given** 重构完成后运行全量集成测试（6 种路由组合 × 非流式/流式 × 纯文本/工具），**When** 执行测试，**Then** 全部 100% 通过，与重构前行为一致。
2. **Given** genai ChatOptions 未直接支持的参数（如 `frequency_penalty`），**When** 适配器将外部请求中的该字段写入 `ChatOptions.extra_body`，**Then** 下游 genai Client 发出的 HTTP 请求体中包含该字段。
3. **Given** 重构后的代码，**When** 统计删除的代码行数，**Then** 所有自定义 HTTP 客户端、请求构建、响应解析、SSE 流解析代码被删除，适配器代码总量减少 40% 以上。

---

### Edge Cases

- genai `ChatOptions.extra_body` 透传参数可能被 genai 自身的 adapter 忽略（如 Anthropic adapter 不支持 extra_body），此时该参数不会出现在下游请求中。适配器应记录 WARN 日志提示参数可能被丢弃。
- genai 的模型名→AdapterKind 推断与 LLM Mux 路由逻辑已解耦：Provider 配置通过 `format` 字段显式指定 genai AdapterKind，LLM Mux 直接构造 `ModelSpec::Target(ServiceTarget)` 绕过 genai 的模型名推断，避免冲突。
- 协议独有的 content block 类型（如 Anthropic 的 `redacted_thinking`）在 genai `ContentPart` 中没有对应变体时，应映射为 `ContentPart::Custom` 并保留原始 payload 用于回写。
- 当后端协议与请求协议相同时（直通场景），IR ↔ genai ↔ API 的双重转换应等价于一次直接 API 调用，不引入新的语义损失。
- 当 Provider 配置的 `format` 值对应的 genai AdapterKind 未在入站协议适配器中实现时（如 `format: gemini`），该后端仅作为出站通道可用——入站仍然通过三种已知端点路径匹配适配器，不影响路由。

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 系统必须以 genai 的类型体系作为统一 IR——`ChatRequest` 表示统一请求，`ChatResponse` 表示统一非流式响应，`ChatStreamEvent` 序列表示统一流式事件。不再维护自有的 IrRequest/IrResponse 自定义抽象。
- **FR-002**: 系统必须将三种外部协议（OpenAI Chat Completions、OpenAI Responses、Anthropic Messages）的入站适配器拆分为独立模块，各自实现从外部 HTTP 请求到 genai IR 的解码和从 IR 到外部 HTTP 响应的编码。入站协议由 HTTP 端点路径自动识别（如 `/v1/chat/completions` → Chat 适配器、`/v1/messages` → Anthropic 适配器、`/v1/responses` → Responses 适配器），不依赖配置中的 `format` 字段。
- **FR-003**: 每个协议适配器必须实现统一的 `Adapter` trait，包含：`decode_request(raw_body) → ChatRequest`、`encode_response(ChatResponse) → response_body`、事件级流式编解码方法。
- **FR-004**: 系统必须移除自有的 HTTP 客户端实现，下游调用统一通过 genai `Client` 发起，使用 `ModelSpec::Target(ServiceTarget)` 精确指定目标端点、认证、模型标识。
- **FR-005**: 路由系统必须将匹配到的后端配置（url、API key、model name）构造为 genai `ServiceTarget`，传递给 genai Client 完成实际调用。Provider 配置中的 `format` 字段直接映射 genai `AdapterKind`，不通过模型名推断或硬编码，确保配置即可驱动 adapter 选择。
- **FR-006**: 对于 genai `ChatOptions` 直接支持的参数（temperature、max_tokens、top_p、stop_sequences、reasoning_effort、response_format、tool_choice、seed、verbosity 等），适配器必须将其映射到 ChatOptions 对应字段而非 extra_body。
- **FR-007**: 对于 genai `ChatOptions` 未直接支持的协议特有参数（如 Chat Completions 的 frequency_penalty、presence_penalty、logit_bias，Anthropic 的 top_k、thinking display 等），适配器必须将其写入 `ChatOptions.extra_body`（或 genai 支持的等效扩展机制）以确保参数透传至下游。
- **FR-008**: 对于 genai `ContentPart` 变体未覆盖的内容类型（如 Anthropic 的 redacted_thinking 块），适配器必须将其映射为 `ContentPart::Custom`，保留原始 payload 供编码阶段完整还原。
- **FR-009**: 重构必须保持现有功能完整：三种协议互转、流式传输、工具调用、JSON Schema 结构化输出、推理控制、Prompt 缓存。已通过 spec 002 定义的全量集成测试在重构后 100% 通过。
- **FR-010**: 重构后代码行数净减少。删除的重复代码（自建 HTTP client、请求构建、响应解析、SSE 解析）应超过新增的适配器 boilerplate。目标：适配器与路由核心逻辑总量相比重构前减少 40% 以上。
- **FR-011**: 适配器模块不得直接依赖 `tokio` / `hyper` / `axum` 等 HTTP 框架——这些由 genai 和路由层统一管理。适配器是纯数据转换逻辑，可独立单元测试。
- **FR-012**: genai 返回的 `genai::Error` 必须被映射为客户端协议格式的错误响应体（如 OpenAI 的 `{"error": {"type": "...", "message": "..."}}`），不同 HTTP 状态码对应正确的协议级错误类型。

### Key Entities

- **genai IR (复用)**: 以 genai crate 的 `ChatRequest`、`ChatResponse`、`ChatStreamEvent`、`MessageContent`、`ContentPart`、`Tool`、`ToolCall`、`ToolResponse`、`ChatOptions`、`Usage` 等类型作为统一 IR，不额外定义中间层。
- **Adapter trait**: 协议适配器的统一接口，定义 `decode_request`、`encode_response`、事件流编解码等方法签名。
- **External Request Types**: 每种协议的原生请求结构（OpenAI Chat 的 `CreateChatCompletionRequest`、Anthropic 的 `CreateMessageRequest`、OpenAI Resp 的 `CreateResponseRequest`）——仅用于适配器内部。
- **Provider Config**: 后端配置条目，以 map 格式组织（`providers: { <name>: { ... } }`），每个条目包含 `format`（genai AdapterKind，如 `openai`/`anthropic`/`openai_resp`）、url、auth、model name。`format` 字段直接映射 genai 的 `AdapterKind`，不通过模型名推断。
- **Protocol extension fields**: 通过 `ChatOptions.extra_body` 和 `ContentPart::Custom` 承载 genai 未覆盖的字段，确保协议保真度。

## Clarifications

### Session 2026-05-28

- Q: Provider 配置如何指定对应的 genai adapter？ → A: 将 provider 配置中的 `protocol` 字段改为 `format` 字段，值直接使用 genai `AdapterKind` 的可序列化名称（如 `openai`、`anthropic`、`openai_resp`），不做二次映射。Provider 配置采用 map 格式（`providers: { <name>: { format, ... } }`）而非数组格式。
- Q: 入站协议适配器 vs genai 出站 Adapter 的关系？ → A: `format` 仅决定出站的 genai adapter；入站协议由 HTTP 端点路径自动识别（如 `/v1/chat/completions` → Chat 适配器），两者独立。入站适配器仅需实现当前三种协议，genai 支持的其余 AdapterKind 可作为出站通道直接使用。
- Q: Crate 命名和组织方式？ → A: 子 crate 文件夹去除 `llm-mux-` 前缀（`core`、`gateway`）。三个入站适配器归入统一 `crates/adapters/` 文件夹管理（`adapters/openai`、`adapters/anthropic`、`adapters/openai-resp`）。

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 重构后所有 spec 002 定义的全量集成测试 100% 通过，不引入功能回归。
- **SC-002**: 适配器与路由核心逻辑代码行数相比重构前减少 40% 以上（删除自建 HTTP client、请求构建、响应解析、SSE 解析代码）。
- **SC-003**: 新增一个协议适配器（如 Gemini）所需的代码行数 ≤ 重构后最小适配器的代码行数（可参考；不在当前交付范围），且仅需实现 Adapter trait，不修改路由核心。
- **SC-004**: 请求协议转换延迟 ≤ 1ms（含 decode + IR 构造，不含网络 I/O），与重构前性能持平或更优。
- **SC-005**: 单个流式事件翻译延迟 ≤ 100μs（与重构前目标一致）。

## Assumptions

- genai crate (v0.6+) 的 public API 足够稳定，其 `ChatRequest`、`ChatResponse`、`ChatOptions`、`ContentPart` 等类型可作为 IR 的基础而不频繁 breaking change。
- genai 的 `extra_body` 机制在 OpenAI-兼容 adapter 中支持将未识别字段合并至请求体；对于不支持 extra_body 的 adapter（如 Anthropic），参数丢失是可接受的降级行为。
- 路由系统逻辑（模型名称匹配、通配符、条件匹配）在重构中基本保持不变，仅将输出从"自定义 HTTP client 调用"改为"构造 ServiceTarget 传递给 genai Client"。
- 现有的 `IrRequest`、`IrResponse`、`IrStreamEvent` 等自定义类型在重构完成后可废弃/删除。
- genai 已封装的 provider 列表（25+ AdapterKind）覆盖了当前和可预见的需要直连的后端类型。对于 genai 支持但未实现入站协议适配器的 provider（如 Gemini、Groq、DeepSeek 等），仍可作为出站后端直接使用——`format` 字段指定 genai AdapterKind 即可，无需额外开发。
- 重构不改变外部 HTTP 接口（同样的端点路径、同样的请求/响应格式），对 LLM Mux 的用户透明。

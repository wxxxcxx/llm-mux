# Feature Specification: LLM Mux 三协议互转网关

**Feature Branch**: `001-tri-protocol-gateway`

**Created**: 2026-05-27

**Status**: Draft

**Input**: User description: "LLM Mux 是一个 Rust 实现的高性能 LLM API 协议互转网关，通过统一 Internal Representation (IR) 将 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种协议双向互转，使得任意 SDK 可调用任意兼容模型。以单一静态二进制交付（<15MB），可作为库嵌入、CLI 独立运行或 Docker 部署。"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 核心双协议互转 (Priority: P1)

开发团队使用 OpenAI Python SDK 连接 LLM Mux 网关，发送 Chat Completions 请求。
LLM Mux 接收 OpenAI 格式请求后，通过统一 IR 将请求转换为 Anthropic Messages 格式，
转发至 Anthropic 兼容后端，并将返回的 Anthropic 格式响应还原为 OpenAI 格式返回给客户端。
反之亦然：使用 Anthropic SDK 的开发者同样可以调用 OpenAI 兼容模型。

开发团队无需修改现有代码、SDK 或工具链。只需将 API Base URL 指向 LLM Mux 网关地址，
即可透明访问其他提供商的模型。

**Why this priority**: 这是核心价值主张——"用你已有的 SDK，调用所有模型"。双协议互转
是实现该目标的最小可行单元，当前 Chat Completions ↔ Messages 编解码和 IR 层已初步实现，
是验证架构可行性和交付第一个可用版本的基础。

**Independent Test**: 启动 LLM Mux 服务，分别使用 OpenAI SDK 和 Anthropic SDK 发送请求，
验证每种 SDK 都能正确调用两种后端的模型并获得语义等价响应。对请求中的模型名称、消息内容、
工具定义、温度参数等进行逐字段验证。

**Acceptance Scenarios**:

1. **Given** LLM Mux 服务已启动并配置 Anthropic 后端，**When** 开发者使用 OpenAI SDK 发送 Chat Completions 请求（含 system prompt、user message、temperature、max_tokens），**Then** LLM Mux 将请求转换为 Anthropic Messages 格式发送至 Anthropic 后端，并将 Anthropic 返回的文本响应正确还原为 OpenAI Chat Completions 格式（含 id、choices、usage 等字段）。

2. **Given** LLM Mux 服务已启动并配置 OpenAI 后端，**When** 开发者使用 Anthropic SDK 发送 Messages 请求（含 system、user content、max_tokens），**Then** LLM Mux 将请求转换为 OpenAI Chat Completions 格式发送至 OpenAI 后端，并将 OpenAI 返回的响应正确还原为 Anthropic Messages 格式。

3. **Given** LLM Mux 已配置 OpenAI 后端，**When** 开发者使用 OpenAI SDK 发送带工具定义（tools + tool_choice）的请求，**Then** LLM Mux 保留工具定义并正确传递至后端，后端返回的 tool_calls 响应被正确还原为 OpenAI 格式。

4. **Given** LLM Mux 已配置 Anthropic 后端，**When** 开发者使用 Anthropic SDK 发送带 tool_use 的请求，**Then** LLM Mux 正确转换为 OpenAI tool_calls 格式，后端返回的工具调用和后续 tool_result 正确还原为 Anthropic format。

---

### User Story 2 - OpenAI Responses 协议接入 (Priority: P2)

开发者使用 OpenAI 最新 Responses API（替代 Chat Completions 的新接口）连接 LLM Mux 网关。
网关正确解析 Responses 协议格式（内置工具调用、结构化输出、状态管理等新特性），
并通过 IR 与其他两种协议（Chat Completions、Messages）进行双向互转。

**Why this priority**: OpenAI Responses API 是 OpenAI 官方推出的新一代接口，逐步替代
Chat Completions API。支持该协议是实现"三种协议全覆盖"承诺的关键环节，也是未来 OpenAI
用户迁移至 LLM Mux 的前提条件。

**Independent Test**: 使用 OpenAI Responses API 格式向 LLM Mux 发送请求，验证请求被正确
翻译为 Chat Completions 和 Messages 格式并返回等价响应；同时验证其他两种协议的请求能
被正确翻译为 Responses 格式。

**Acceptance Scenarios**:

1. **Given** LLM Mux 已配置 Anthropic 后端，**When** 开发者使用 OpenAI Responses API 格式发送请求（含 instructions、input、tools），**Then** LLM Mux 将请求转换为 Anthropic Messages 格式并返回等价响应。

2. **Given** LLM Mux 已配置 OpenAI 后端，**When** 开发者使用 Anthropic SDK 发送 Messages 请求，**Then** LLM Mux 可配置将响应编码为 OpenAI Responses 格式返回给客户端。

3. **Given** LLM Mux 支持全部三种协议，**When** 开发者使用任意协议的 SDK 发送请求，**Then** 请求可被路由到任意两种其他协议的后端并返回正确的语义等价响应（全部 6 种路由组合通过测试）。

---

### User Story 3 - HTTP 服务与部署 (Priority: P3)

运维人员下载 LLM Mux 单一静态二进制文件（<15MB），通过 CLI 命令行参数（如 `--port`、
`--config`）或配置文件启动 HTTP 服务。也可将二进制打包为 Docker 镜像部署至容器编排平台，
或在 Rust 项目中作为库嵌入使用。

**Why this priority**: 交付体验直接影响用户采用门槛。单一二进制、零依赖部署是 Rust 工具的
标志性优势，CLI + Docker + 库嵌入三种交付形态覆盖从个人开发者到企业部署的全场景。

**Independent Test**: 在不同平台上（Linux、macOS）下载二进制，通过 CLI 启动服务，用 curl
验证 HTTP 端点可访问。验证 Docker 镜像可构建并正常响应请求。验证作为 Rust 库嵌入时可在
其他项目中调用。

**Acceptance Scenarios**:

1. **Given** 用户下载 LLM Mux 二进制，**When** 执行 `./llm-mux --port 8080 --config config.yaml`，**Then** HTTP 服务在 8080 端口启动，`/v1/chat/completions` 端点可接受请求。

2. **Given** Docker 环境已就绪，**When** 用户执行 `docker run -p 8080:8080 llm-mux`，**Then** 容器内 LLM Mux 服务启动并可正常处理请求。

3. **Given** Rust 开发者引入 `llm-mux` 为依赖，**When** 在代码中配置路由规则并启动服务，**Then** 服务在应用进程内正常运行，与其他应用逻辑共存。

4. **Given** 编译的二进制文件，**When** 检查文件大小，**Then** 二进制大小不超过 15MB（Release 构建，含 debuginfo stripped）。

---

### User Story 4 - 全协议流式传输 (Priority: P4)

开发者请求流式响应（SSE），LLM Mux 在接收后端 SSE 流时，实时逐事件翻译并推送给客户端，
不缓冲完整响应。三种协议（Chat Completions、Responses、Messages）的流式请求均被支持。

**Why this priority**: 流式传输是 LLM 应用的标配体验（逐字输出）。实现逐事件零缓冲翻译
对延迟敏感场景（聊天、实时生成）至关重要。在核心互转（P1）完成后，流式能力是自然的扩展。

**Independent Test**: 以任意协议发送 `stream: true` 请求，验证 SSE 事件被正确翻译并逐事件
推送至客户端，且端到端首 token 延迟与直接调用后端相比增加不超过 100ms。

**Acceptance Scenarios**:

1. **Given** LLM Mux 服务运行中，**When** 开发者使用 OpenAI SDK 发送 `stream: true` 的 Chat Completions 请求，后端为 Anthropic，**Then** 客户端逐 token 收到 OpenAI 格式的 SSE 流，中途无缓冲、无截断。

2. **Given** LLM Mux 服务运行中，**When** 后端流异常中断，**Then** LLM Mux 将错误以客户端协议格式编码为 SSE 错误事件发送给客户端，并正确关闭连接。

3. **Given** 全部三种协议均已接入，**When** 以任意协议的流式模式发送请求，路由到任意其他协议的后端，**Then** 6 种流式路由组合均能正确完成流式传输（事件格式、字段映射、终止信号均正确）。

---

### Edge Cases

- 当后端返回非标准 HTTP 状态码（如 429、502）时，LLM Mux 应将错误映射为客户端的协议格式（如 OpenAI 的 `error` 对象），而非裸 HTTP 错误。
- 当请求包含未知字段（providers 未来新特性）时，LLM Mux 应透传而非丢弃这些字段，确保前向兼容。
- 当工具调用循环发生时（多轮 tool_use / tool_result 交互），LLM Mux 应正确追踪和映射每轮的工具 ID 和结果。
- 当请求体极大（如含多张 base64 图片的视觉请求）时，不应因内存复制导致超时或 OOM。
- 当后端 SSE 流发送非标准事件类型时，LLM Mux 应安全捕获并以客户端协议格式降级处理，而非崩溃。
- 当配置文件格式错误或缺失时，CLI 应给出明确的错误信息和使用说明，而非静默失败。
- 当 LLM Mux 进程崩溃或重启时，客户端收到连接断开，由客户端 SDK 重试逻辑处理；网关不负责请求重放或流恢复。

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 系统必须支持 OpenAI Chat Completions 请求格式，解析模型名称、消息列表、采样参数（temperature、top_p 等）、工具定义（tools、tool_choice）、流式标志（stream）等字段。
- **FR-002**: 系统必须支持 Anthropic Messages 请求格式，解析 model、system、messages、max_tokens、tools、tool_choice、stream 等字段。
- **FR-003**: 系统必须支持 OpenAI Responses API 请求格式，解析 instructions、input、tools、stream 等字段，以及 Responses 特有的内置工具调用、状态管理和结构化输出。
- **FR-004**: 系统必须提供统一 Internal Representation (IR)，将三种协议格式标准化为单一的请求/响应/流事件模型，实现协议无关的内部处理。
- **FR-005**: 系统必须支持三种协议之间的任意双向互转——任一协议的请求可路由至任一其他协议的后端，响应正确还原为原始请求协议的格式（共 6 种路由组合）。
- **FR-006**: 系统必须支持完整的请求-响应字段映射，包括但不限于：文本内容、工具调用/结果、思考过程（thinking/reasoning）、引用（citations）、图片内容、Token 用量统计、停止原因（stop_reason/finish_reason）。
- **FR-007**: 系统必须提供基于 API Key 的认证验证机制，拒绝未授权请求。
- **FR-008**: 系统必须支持可配置的路由规则：基于模型名称（支持通配符）、入站协议（protocol）、流式标志（stream）、工具存在（has_tools）、媒体存在（has_media）等条件的多维度 AND 匹配，从上到下首个命中生效；兜底规则 `models: ["*"]` 必须放在最后。
- **FR-009**: 系统必须以单一静态二进制文件形式交付（Release 构建 < 15MB），支持 Linux 和 macOS 平台。
- **FR-010**: 系统必须提供 CLI 接口，支持通过命令行参数（--port、--config、--log-level）和配置文件两种方式配置服务。
- **FR-011**: 系统必须提供 Docker 部署支持，包含官方 Dockerfile。
- **FR-012**: 系统必须以 Rust 库形式发行（crate），允许其他 Rust 项目嵌入 LLM Mux 核心功能。
- **FR-013**: 系统必须支持 SSE 流式传输，以逐事件方式翻译流式响应，不缓冲完整响应体。当后端流暂停时，客户端必须在 100ms 内感知到流中断（背压传播验证标准）。
- **FR-014**: 系统必须将后端错误响应（HTTP 错误、协议错误、流中断）映射为客户端协议格式的错误表示。
- **FR-015**: 系统必须透传未知字段（provider extension fields），确保前向兼容性。
- **FR-016**: 系统必须使用结构化日志记录请求/响应元数据（timestamp、request_id、model、协议、延迟（毫秒）、状态码、Token 用量），日志级别可通过 `--log-level` 配置（支持 error、warn、info、debug、trace），默认 INFO。当请求延迟超过 p95 基线 2 倍时，以 WARN 级别记录。
- **FR-017**: 首版不实现内置速率限制；并发控制和 QPS 限制由上游反向代理（nginx/envoy/Caddy）或容器编排平台（K8s Ingress）处理。
- **FR-018**: 系统必须为每个入站请求生成唯一 Request ID，注入下游请求并在结构化日志中记录；不依赖客户端或后端提供 ID。
- **FR-019**: 系统必须支持优雅关闭：收到 SIGTERM 信号后停止接受新请求，在可配置的 drain 超时（默认 30s）内完成进行中请求后退出。
- **FR-020**: 系统必须提供 `/health` 端点，返回服务就绪状态，供负载均衡器和容器编排系统进行健康检测。
- **FR-021**: 系统必须支持可配置的后端 HTTP 连接参数：连接池大小（默认 128）、connect timeout（默认 5s）、read timeout（默认 300s 以适应流式长连接）、keepalive idle timeout（默认 90s）。
- **FR-022**: 编解码热路径（decode_request / encode_request / decode_stream_event / encode_stream_event）必须最小化堆分配，每个事件的处理不得引入超过 O(1) 的额外分配。

### Key Entities

- **IrRequest**: 统一内部请求格式，包含 model、messages（标准化为 ContentBlock 序列）、采样参数、工具定义、流式标志、provider_extensions（透传字段容器）。
- **IrResponse**: 统一内部响应格式，包含 id、model、content（ContentBlock 序列）、stop_reason、usage（Token统计）、provider_extensions。
- **IrStreamEvent**: 统一内部流事件格式，表示单个 SSE 事件（text_delta、tool_use_delta、error 等事件类型）。
- **ContentBlock**: 内容块类型，统一表示文本、思考过程、工具调用、工具结果、图片、拒绝响应、文档引用等多种内容形式。
- **Protocol**: 协议枚举，标识三种外部协议（OpenAI Chat、OpenAI Responses、Anthropic Messages）。
- **RouteRule**: 路由规则，定义基于模型名称、工具使用、媒体存在等条件的后端匹配逻辑。
- **ProtocolConversion**: 协议转换规则，定义不同协议间字段的映射关系（如 Chat 的 `finish_reason: "tool_calls"` ↔ Anthropic 的 `stop_reason: "tool_use"`）。

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 开发者使用 OpenAI SDK 发送请求，所有字段（消息内容、工具定义、采样参数、Token 用量）被正确翻译并返回，交叉协议回归测试 100% 通过（全部 6 种路由组合）。
- **SC-002**: 请求协议转换延迟 ≤ 1ms（典型请求体 < 10KB，测量范围：纯编解码逻辑，不含网络 I/O 和序列化开销），单个流式事件转换延迟 ≤ 100μs（同测量范围）。
- **SC-003**: Release 构建的二进制文件大小 < 15MB（Linux x86_64 stripped）；macOS aarch64 和 Linux aarch64 ≤ 20MB（受目标平台工具链差异影响，宽松容差）。
- **SC-004**: 开发者从下载二进制到成功发送第一个请求的时间 ≤ 5 分钟（包含解压、配置、启动）。
- **SC-005**: 端到端首 token 延迟增量（通过 LLM Mux 与直连后端对比）≤ 100ms。
- **SC-006**: 三种协议的流式传输均正确完成，无事件丢失、格式错误或提前终止。
- **SC-007**: LLM Mux 服务在 100 并发请求下（50% 流式 + 50% 非流式，平均请求体 5KB），p95 翻译延迟 ≤ 2ms，内存占用 ≤ 50MB。
- **SC-008**: 非流式请求端到端响应延迟增量（通过 LLM Mux 与直连后端对比）≤ 50ms。

## Clarifications

### Session 2026-05-27

- Q: 每个协议应锁定到哪个 API 版本？ → A: 锁定当前已实现版本（OpenAI Chat `/v1/chat/completions`、Anthropic Messages `2023-06-01`、OpenAI Responses `/v1/responses` 首个稳定版），后续版本通过配置切换。
- Q: 请求 ID 应如何跨 LLM Mux 传播？ → A: 网关为每个入站请求生成唯一 Request ID，注入下游请求并记录日志，不依赖客户端或后端提供 ID。
- Q: LLM Mux 自身崩溃时的处理期望？ → A: 结合 A+C——收到 SIGTERM 时在 drain 超时内完成进行中请求后优雅退出；提供 `/health` 端点供负载均衡器检测，依赖 Docker/K8s 自动重启，网关自身不做请求重放或状态恢复。
- Q: 访问日志范围和字段要求？ → A: 使用结构化日志框架，支持日志级别设置（如 `--log-level`），默认 INFO 级别。
- Q: 是否需要网关内置速率限制？ → A: 首版不实现内置速率限制，由上游反向代理或 K8s Ingress 处理。

## Assumptions

- 下游 LLM 后端提供标准兼容的 API 端点（OpenAI Chat 或 Anthropic Messages 格式）。
- 用户已持有有效的后端 API Key，LLM Mux 负责透传和验证，不管理 API Key 的生命周期。
- 网络延迟（LLM Mux ↔ 后端）不在网关控制范围内，性能指标仅衡量网关自身的处理开销。
- 暂不支持非标准/私有协议扩展。初始版本锁定以下 API 版本：OpenAI Chat Completions `/v1/chat/completions`、Anthropic Messages `2023-06-01`、OpenAI Responses `/v1/responses` 首个稳定版。后续版本切换通过配置实现。
- 配置文件格式使用 YAML，与 Rust 生态常用的 `serde_yaml` 兼容。
- 目标运行时环境为 Linux（x86_64、aarch64）和 macOS（x86_64、aarch64），Windows（x86_64、aarch64）。
- Docker 镜像基于 `scratch` 或 `distroless` 基础镜像以最小化体积。

---
title: 'LLM Mux — LLM API 协议互转网关'
status: final
created: 2026-05-29
updated: 2026-05-29
---

# PRD: LLM Mux — LLM API 协议互转网关

## 0. Document Purpose

本 PRD 面向开发团队和后续迭代规划，描述 LLM Mux 的现有能力、架构决策和未来方向。基于已有代码库和功能规格反推，作为棕地项目的需求基线文档。后续下游工作流（UX 设计、架构文档、史诗/故事拆分）均以此文档为输入。

## 1. Vision

LLM Mux 是一个高性能 LLM API 协议互转网关。核心价值：**开发者用已有的 SDK 和工具链，透明调用任意后端的 LLM 模型。**

通过统一 Internal Representation（IR）层，将 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种入站协议双向互转到 25+ 种出站后端（OpenAI、Anthropic、Gemini、Groq、DeepSeek 等 genai 支持的 AdapterKind）。以单一静态二进制交付（< 15MB），支持 CLI、Docker、库嵌入三种部署形态。

解决了"模型锁定"问题——团队不需要为每个模型更换 SDK，不需要在代码中维护多套协议适配逻辑。LLM Mux 作为透明代理，让"换模型"变成配置文件的一行改动。

## 2. Target User

### 2.1 Jobs To Be Done

- **开发者**：我想用我习惯的 SDK（OpenAI / Anthropic）调用任意模型，不想为每个厂商换一套工具链
- **平台团队**：我想在团队内部统一 LLM 接入层，集中管理 API Key、路由策略和模型映射
- **运维人员**：我想部署一个轻量级网关作为 LLM 请求的唯一入口，不引入额外的运行时依赖（Java、Node 等）

### 2.2 Non-Users (v1)

- 非技术用户：LLM Mux 没有图形界面，不提供聊天 UI
- 需要内置限流/计费/用量分析的用户：这些功能由上层反向代理或网关编排层处理
- 需要模型训练/微调能力的用户：LLM Mux 仅做推理请求转发

### 2.3 Key User Journeys

- **UJ-1. 开发者用 OpenAI SDK 调用 Claude 模型。**
  - **Persona + context:** 后端工程师 Alice，团队使用 OpenAI SDK，但希望试用 Claude 的能力。
  - **Entry state:** Alice 本地开发环境，已安装 `openai` Python SDK。
  - **Path:**
    1. 下载 LLM Mux 二进制，生成默认配置
    2. 填入 Anthropic API Key，配置 `gpt-4o -> claude-sonnet-4-6` 模型映射
    3. 启动服务
    4. 将 OpenAI SDK 的 `base_url` 改为 `http://localhost:8080/v1`
    5. 发送请求，`model="gpt-4o"` → 实际调用 `claude-sonnet-4-6`
  - **Climax:** 收到 Anthropic 模型的响应，格式与 OpenAI 完全一致
  - **Resolution:** Alice 不需要改一行代码就完成了模型切换

- **UJ-2. 平台团队统一管理多模型路由。**
  - **Persona + context:** 基础架构工程师 Bob，需要为团队搭建统一的 LLM 接入层。
  - **Entry state:** 团队内有 OpenAI、Anthropic、Gemini 等多个模型在使用。
  - **Path:**
    1. 配置多级路由规则（`claude-*` → Anthropic, `gemini-*` → Gemini, `*` → OpenAI）
    2. 配置 API Key 白名单
    3. 配置模型名映射（简化团队认知负担）
    4. 以 Docker 形式部署到 K8s
  - **Climax:** 团队所有成员的请求经过统一网关，路由和 API Key 管理集中化
  - **Resolution:** Bob 可以在不通知团队的情况下调整路由策略

## 3. Glossary

- **IR (Internal Representation)** — 网关内部的统一请求/响应/流事件模型。当前复用 genai crate 的 `ChatRequest`、`ChatResponse`、`ChatStreamEvent` 等类型体系。
- **入站协议** — 客户端向网关发起请求时使用的 API 协议。支持三种：OpenAI Chat Completions、OpenAI Responses、Anthropic Messages。
- **出站后端** — 网关转发的目标 LLM 服务。由配置中的 `format` 字段指定 genai `AdapterKind`（如 `openai`、`anthropic`、`gemini` 等）。
- **Adapter** — 协议适配器 trait，定义 `decode_request`、`encode_response`、`encode_stream_event`、`encode_error` 四个方法。每种入站协议独立实现。
- **Router** — 路由引擎，基于模型名（glob）、入站协议、是否流式、是否有工具、是否有媒体等多条件自上而下匹配，首个命中生效。
- **Provider** — 出站后端配置条目，包含 `format`、`url`、`api_key`、`headers`、`model_mapping`。
- **extra_body** — genai `ChatOptions` 的逃逸机制，用于透传 genai 未直接支持的协议特有参数（如 `frequency_penalty`、`top_k`、`stop_sequences`）。
- **SSE (Server-Sent Events)** — 流式传输协议，LLM Mux 逐事件翻译后端流事件并推送给客户端。

## 4. Features

### 4.1 三协议入站解析

**Description:** LLM Mux 支持三种主流 LLM API 协议的请求解析。每种协议的入站适配器独立实现，通过 HTTP 端点路径自动识别（`/v1/chat/completions` → Chat、`/v1/messages` → Anthropic、`/v1/responses` → Responses）。Realizes UJ-1, UJ-2.

**Functional Requirements:**

#### FR-1: OpenAI Chat Completions 解析

系统可以解析 OpenAI Chat Completion Request 格式，包括 messages（system/user/assistant/tool role）、tools/tool_choice、采样参数（temperature、top_p、max_tokens、stop）、流式标志（stream）、response_format（json_object/json_schema）。Realizes UJ-1.

**Consequences (testable):**
- 请求中的 system/developer role 消息被提取为 system prompt
- assistant 消息中的 tool_calls 被正确解析
- tool 角色消息关联正确的 tool_call_id
- 多模态 `content: [{type: "image_url", ...}]` 被解析为二进制内容

#### FR-2: Anthropic Messages 解析

系统可以解析 Anthropic Messages Request 格式，包括 system（文本/多段块）、messages（user/assistant role）、tools/tool_choice、thinking、max_tokens。Realizes UJ-1.

**Consequences (testable):**
- `system` 字段被提取为 system prompt（支持文本和多段块两种格式）
- `thinking` 配置被提取为推理配置（mode + budget_tokens）
- 多模态 `content: [{type: "image", source: {...}}]` 被解析为二进制内容

#### FR-3: OpenAI Responses 解析

系统可以解析 OpenAI Responses API Request 格式，包括 instructions、input、tools、stream、response_format、tool_choice、max_output_tokens、temperature、top_p、store、metadata、previous_response_id。Realizes UJ-1.

**Consequences (testable):**
- `instructions` 字段被映射为 system prompt
- `input` 中的消息序列被正确映射
- 识别 Responses API 独有的字段

#### FR-4: 入站协议自动识别

入站协议由 HTTP 端点路径自动识别，无需配置。`/v1/chat/completions` → OpenAI Chat，`/v1/messages` → Anthropic Messages，`/v1/responses` → OpenAI Responses。Realizes UJ-2.

**Consequences (testable):**
- 每个端点只接受对应协议的请求体格式
- 端点路径硬编码而非可配置

### 4.2 跨协议响应编码

**Description:** 三种入站协议的响应编码方向。genai 返回的 `ChatResponse`/`ChatStreamEvent` 被编码为客户端原始请求协议的格式，包括响应体、流式事件、错误响应。Realizes UJ-1.

#### FR-5: OpenAI Chat 响应编码

系统可以将内部 IR 编码为 OpenAI Chat Completion Response JSON，包含 id、object、created、model、choices（message.content/finish_reason）、usage。流式事件编码为 `data: {...}\n\n` SSE 格式，以 `data: [DONE]` 终止。Realizes UJ-1.

**Consequences (testable):**
- `finish_reason` 正确映射（end_turn→stop, tool_use→tool_calls, max_tokens→length, content_filter→content_filter）
- 工具调用正确还原为 tool_calls 格式
- 流式 SSE 格式合规（data: 前缀、[DONE] 终止）

#### FR-6: Anthropic Messages 响应编码

系统可以将内部 IR 编码为 Anthropic Messages Response JSON，包含 id、type、role、model、content（text/tool_use blocks）、stop_reason、usage。流式事件编码为 SSE，自动注入 `message_start` + `content_block_start` 事件，以 `message_stop` 终止。Realizes UJ-1.

**Consequences (testable):**
- `stop_reason` 正确映射（end_turn→end_turn, tool_use→tool_use, max_tokens→max_tokens）
- 工具调用正确还原为 tool_use content block
- 流式事件序列完整：message_start → content_block_start → content_block_delta → content_block_stop → message_delta → message_stop

#### FR-7: OpenAI Responses 响应编码

系统可以将内部 IR 编码为 OpenAI Responses Response JSON。流式事件编码为 SSE，使用 `response.output_text.delta` 和 `response.completed` 事件类型。

**Consequences (testable):**
- SSE 事件类型使用 Responses API 的命名规范
- 非流式响应体结构正确

#### FR-8: 错误响应编码

系统可以将内部错误（解码失败、路由失败、后端错误）编码为客户端协议的格式。Realizes UJ-1.

**Consequences (testable):**
- OpenAI Chat 格式：`{"error": {"message": "...", "type": "invalid_request_error", "code": "500"}}`
- Anthropic Messages 格式：`{"type": "error", "error": {"type": "invalid_request_error", "message": "..."}}`

### 4.3 可配置路由

**Description:** 基于配置文件的路由引擎，支持多条件匹配和模型名映射。Realizes UJ-2.

#### FR-9: 多条件路由匹配

路由规则支持以下条件的 AND 组合匹配：模型名（支持 `*`/`?` glob 通配符）、入站协议、是否流式、是否有工具、是否有媒体。自上而下首个命中生效，兜底规则 `models: ["*"]` 必须无条件放在最后。Realizes UJ-2.

**Consequences (testable):**
- 通配符 `claude-*` 匹配 `claude-sonnet-4-6`
- 精确匹配优先于通配符匹配
- 兜底规则捕获所有未匹配请求
- 路由规则校验：兜底规则不能有条件

#### FR-10: 模型名映射

系统支持配置级的模型名重写，入站模型名 → 后端实际模型名，支持精确映射和通配符映射。Realizes UJ-1, UJ-2.

**Consequences (testable):**
- `gpt-4o → claude-sonnet-4-6` 映射后后端收到 `claude-sonnet-4-6`
- 响应中保持原始入站模型名
- 通配符映射按匹配长度优先

### 4.4 流式传输

**Description:** 系统以逐事件方式翻译流式响应，不缓冲完整响应体。Realizes UJ-1.

#### FR-11: 流式请求处理

系统将 `stream: true` 的请求路由到 genai `exec_chat_stream()`，将返回的 `ChatStream` 逐一翻译为客户端协议格式的 SSE 事件并推送给客户端。Realizes UJ-1.

**Consequences (testable):**
- 事件序列完整，无事件丢失或乱序
- 使用 `futures::StreamExt::next()` 逐事件处理，不预读缓冲
- 流中断时推送 SSE error 事件并关闭连接

#### FR-12: 流式 SSE 格式合规

SSE 输出严格遵循 SSE 规范：每个事件以 `data:` 前缀开头后跟 JSON，事件之间以空行分隔，终止信号格式正确。

**Consequences (testable):**
- OpenAI Chat：以 `data: [DONE]` 终止
- Anthropic Messages：自动注入 `message_start` 和 `content_block_start` 事件
- 每个数据行不包含多余的空白字符

### 4.5 认证与安全

#### FR-13: API Key 认证

系统支持可选的 API Key 白名单认证。配置 `api_keys` 后，所有入站请求必须提供匹配的 API Key；不配置则跳过认证。Authenticator trait 支持扩展。Realizes UJ-2.

**Consequences (testable):**
- 有效 API Key → 请求放行
- 无效 API Key → 401 Unauthorized
- 空 `api_keys` → 所有请求放行

### 4.6 运维与部署

#### FR-14: CLI 管理接口

系统提供 CLI 接口：`llm-mux start`（可选的 port/host/config/log-level/daemon/pid-file 参数）、`llm-mux stop`（PID 文件管理）、`llm-mux config init`（生成默认配置）、`llm-mux config validate`、`llm-mux config show`。Realizes UJ-2.

**Consequences (testable):**
- 所有 CLI 子命令有 `--help` 输出
- `config validate` 校验 provider 存在性、路由规则完整性、兜底规则约束
- `start` 支持 daemon 模式（Unix fork + setsid）

#### FR-15: Docker 部署

系统提供 Dockerfile 多阶段构建：builder 使用 `rust:1.85-slim-bookworm`，运行时使用 `gcr.io/distroless/cc-debian12`。最终二进制 < 15MB。Realizes UJ-2.

**Consequences (testable):**
- Docker 构建成功，最终镜像 < 20MB
- 容器以 nobody 用户运行（65534:65534）
- 默认监听 8080 端口

#### FR-16: 健康检查

系统提供 `GET /health` 端点，返回 `{"status": "ok"}` 200 OK。Realizes UJ-2.

#### FR-17: 优雅关闭

系统收到 SIGTERM（Unix）或 Ctrl+C（非 Unix）后停止接受新请求，在可配置的 drain 超时（默认 30s）内完成进行中请求后退出。Realizes UJ-2.

**Consequences (testable):**
- 优雅关闭日志输出 "shutdown signal received, draining for {n}s..."
- 超时后进程强制退出

#### FR-18: 请求 ID

系统为每个入站请求生成 UUID v7 作为 Request ID，注入 tracing span 并在响应头 `X-Request-ID` 中返回。Realizes UJ-2.

#### FR-19: 结构化日志

系统使用 `tracing` + `tracing-subscriber` 记录结构化日志，支持 json 格式（非 TTY）和 compact 格式（TTY），日志级别可配置。Realizes UJ-2.

## 5. Non-Goals (Explicit)

- **不是 API 聚合器/市场** — 不提供模型对比、价格聚合、可用性监控等功能
- **不是 API 管理平台** — 不提供用量统计、计费、用户管理、速率限制
- **不是模型训练/微调平台** — 仅做推理请求转发
- **不是 Chat UI** — 不提供聊天界面
- **不实现内置速率限制** — 由上游反向代理或 K8s Ingress 处理
- **不管理 API Key 生命周期** — 仅透传和验证，不负责 Key 的签发、轮换、过期
- **不保证请求重放或流恢复** — 网关崩溃后不恢复状态

## 6. MVP Scope

### 6.1 In Scope

- 三种入站协议解析与编码（Chat、Messages、Responses）
- 6 种跨协议路由组合
- 非流式 + 流式请求处理
- 工具调用双向互转
- 多模态内容（图片、文档）
- 可配置路由（通配符 + 多条件）
- 模型名映射
- API Key 认证
- CLI + Docker 部署
- 健康检查 + 优雅关闭
- 结构化日志 + 请求 ID 追踪
- 协议特有字段透传（extra_body）
- 推理控制（thinking/reasoning）
- 结构化输出（JSON Schema）

### 6.2 Out of Scope for MVP

- Responses API 独有的会话管理（`previous_response_id` `store` 等高级特性）— [NON-GOAL for MVP]
- GraphQL/gRPC 入站协议 — [v2 候选]
- 可观测性集成（Prometheus metrics、OpenTelemetry）— [v2 候选]
- 配置热加载 — [v2 候选]
- 管理 API / Dashboard — [v2 候选]

## 7. Success Metrics

**Primary**
- **SM-1**: 6 种跨协议路由组合的全量集成测试 100% 通过。Validates FR-1–FR-8.
- **SM-2**: Release 构建二进制 < 15MB（Linux x86_64 stripped）。Validates FR-15.
- **SM-3**: 请求协议转换延迟 ≤ 1ms（不含网络 I/O）。Validates FR-1–FR-3.

**Secondary**
- **SM-4**: 流式事件转换延迟 ≤ 100μs/event。Validates FR-11.
- **SM-5**: Docker 构建一次成功，最终镜像 < 20MB。Validates FR-15.

## 8. Open Questions

1. OpenAI Responses 协议的会话管理（`previous_response_id`）在跨协议路由到非 Responses 后端时的行为是否需要特殊处理？
2. 是否需要为 genai 新增的 AdapterKind 自动添加对应的 Provider 配置模板？
3. 后续是否要支持配置热加载（文件变更自动重载路由规则）？

## 9. Assumptions Index

- genai crate (v0.6+) 的 public API 足够稳定，可作为 IR 的基础
- genai 的 `extra_body` 机制在 OpenAI-兼容 adapter 中支持参数透传；不支持 extra_body 的 adapter（如 Anthropic）参数丢失可接受
- genai 支持的 25+ AdapterKind 覆盖了当前和可预见的出站后端需求
- 入站协议路径与 genai adapter 的出站 `format` 字段独立——入站仅支持三种协议，出站支持所有 AdapterKind
- 兜底路由规则 `models: ["*"]` 必须无条件放在最后，否则配置校验不通过

---

## Cross-Cutting NFRs

### Performance

- **NFR-1**: 请求协议转换延迟 ≤ 1ms（典型 < 10KB body，不含网络 I/O）
- **NFR-2**: 流式事件翻译延迟 ≤ 100μs/event
- **NFR-3**: 100 并发请求下（50% 流式）p95 翻译延迟 ≤ 2ms
- **NFR-4**: 内存占用 ≤ 50MB（稳态）

### Security

- **NFR-5**: 代码禁止使用 unsafe（`unsafe_code = "forbid"`）
- **NFR-6**: API Key 在日志中脱敏显示（前 8 字符 + `***`）

### Reliability

- **NFR-7**: 后端异常（HTTP 5xx、超时、非 JSON 响应、流中断）不导致网关崩溃
- **NFR-8**: 优雅关闭在 drain 超时内完成进行中请求

### Observability

- **NFR-9**: 每个入站请求记录结构化日志：request_id、model、protocol、延迟、状态码、Token 用量
- **NFR-10**: 慢请求（超过 p95 基线 2 倍）以 WARN 级别记录

### Compatibility

- **NFR-11**: 支持 Linux x86_64/aarch64、macOS x86_64/aarch64
- **NFR-12**: 未知字段透传，确保前向兼容

## Constraints and Guardrails

### Technical Constraints

- **C-1**: 必须使用 Rust edition 2021，workspace resolver = "2"
- **C-2**: Release 构建使用 LTO、codegen-units=1、opt-level="z"、strip
- **C-3**: 依赖 genai 作为 LLM 客户端层，不自建 HTTP 客户端
- **C-4**: 单文件不超过 400 行，方法不超过 80 行

### Safety

- **C-5**: API Key 来自配置文件或环境变量，不支持运行时动态注入
- **C-6**: 认证默认关闭（空 api_keys=不验证），配置后立即生效

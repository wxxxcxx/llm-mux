---
stepsCompleted: [1, 2, 3, 4]
status: 'complete'
completedAt: '2026-05-29'
inputDocuments:
  - "_bmad-output/planning-artifacts/prds/prd-llm-mux-2026-05-29/prd.md"
  - "_bmad-output/planning-artifacts/architecture.md"
---

# llm-mux - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for llm-mux, decomposing the requirements from the PRD and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

- **FR-1**: OpenAI Chat Completions 请求解析
- **FR-2**: Anthropic Messages 请求解析
- **FR-3**: OpenAI Responses 请求解析
- **FR-4**: 入站协议自动识别（HTTP 路径绑定）
- **FR-5**: OpenAI Chat 响应编码（含 SSE 流式）
- **FR-6**: Anthropic Messages 响应编码（含 SSE 流式）
- **FR-7**: OpenAI Responses 响应编码（含 SSE 流式）
- **FR-8**: 错误响应编码（三种协议错误格式）
- **FR-9**: 多条件路由匹配（模型名 glob + 协议/流式/工具/媒体条件）
- **FR-10**: 模型名映射（精确 + 通配符）
- **FR-11**: 流式请求处理（逐事件翻译，零缓冲）
- **FR-12**: 流式 SSE 格式合规（data: 前缀、[DONE] 终止）
- **FR-13**: API Key 白名单认证
- **FR-14**: CLI 管理接口（start/stop/config）
- **FR-15**: Docker 多阶段构建
- **FR-16**: 健康检查端点
- **FR-17**: 优雅关闭
- **FR-18**: 请求 ID（UUID v7）
- **FR-19**: 结构化日志

### NonFunctional Requirements

- **NFR-1**: 请求协议转换延迟 ≤ 1ms
- **NFR-2**: 流式事件翻译延迟 ≤ 100μs/event
- **NFR-3**: 100 并发下 p95 翻译延迟 ≤ 2ms
- **NFR-4**: 内存占用 ≤ 50MB
- **NFR-5**: 禁止 unsafe 代码
- **NFR-6**: API Key 日志脱敏
- **NFR-7**: 后端异常不导致网关崩溃
- **NFR-8**: 优雅关闭在 drain timeout 内完成
- **NFR-9**: 结构化日志含 request_id/model/protocol/延迟/状态码/Token 用量
- **NFR-10**: 慢请求 WARN 级别记录
- **NFR-11**: 支持 Linux/macOS/Windows
- **NFR-12**: 未知字段透传

### Additional Requirements

- 自定义 IR 中间层（IrRequest/IrResponse/IrStreamEvent），与 genai 类型解耦
- Codec trait 为主接口，adapter 不依赖 genai
- genai 仅作为下游调用层

### UX Design Requirements

_不适用（后端 CLI 工具，无 UI）_

### FR Coverage Map

FR-1~4（入站解析）: Epic 1
FR-5~8（响应编码）: Epic 1
FR-9~10（路由）: Epic 1
FR-11~12（流式）: Epic 1
FR-13（认证）: Epic 1
FR-14~19（运维）: Epic 1

## Epic List

### Epic 1: 核心网关 MVP
核心 LLM API 协议互转网关功能。三协议入站解析与响应编码、可配置路由、流式 SSE 代理、API Key 认证、CLI 管理、Docker 部署、可观测性基础。
**FRs 覆盖:** FR-1~19
**状态:** ✅ 已实现

#### Story 1.1: 入站协议编解码

As a 开发者,
I want 三种协议请求被正确解析、响应被正确编码,
So that 任意 SDK 可通过统一网关调用任意后端。

**Acceptance Criteria:**

**Given** 网关已启动
**When** 发送 OpenAI Chat/Anthropic Messages/OpenAI Responses 格式请求
**Then** 请求被正确解码为自定义 IrRequest
**And** 响应被正确编码回原始请求协议格式

---

#### Story 1.2: 可配置路由

As a 平台工程师,
I want 基于模型名/协议/流式/工具/媒体多条件匹配的路由规则,
So that 请求被分发到正确的后端模型。

**Acceptance Criteria:**

**Given** 配置了多级路由规则
**When** 发送匹配不同规则的请求
**Then** 请求被路由到对应的 provider
**And** 兜底规则 `*` 捕获所有未匹配请求

---

#### Story 1.3: 流式 SSE 代理

As a 开发者,
I want 流式请求被逐事件翻译并推送,
So that 获得实时的逐 token 输出体验。

**Acceptance Criteria:**

**Given** 发送 `stream: true` 的请求
**When** 后端返回流式响应
**Then** 事件被逐事件翻译，不缓冲完整响应
**And** SSE 格式正确，以 `data: [DONE]` 或 `message_stop` 终止

---

#### Story 1.4: CLI 与部署

As a 运维人员,
I want CLI 管理接口 + Docker 部署,
So that 网关可快速启动和上线。

**Acceptance Criteria:**

**Given** 下载二进制或构建 Docker 镜像
**When** 执行 `llm-mux start` 或 `docker run`
**Then** 服务在指定端口启动，`/health` 返回 200

---

#### Story 1.5: 认证与安全

As a 平台工程师,
I want API Key 白名单认证,
So that 未授权请求被拒绝。

**Acceptance Criteria:**

**Given** 配置了 `api_keys`
**When** 发送不带有效 API Key 的请求
**Then** 返回 401 Unauthorized
**And** 带有效 Key 的请求正常放行

---

#### Story 1.6: 可观测性基础

As a 运维人员,
I want 结构化日志 + 请求 ID 追踪,
So that 请求链路可追踪和排查。

**Acceptance Criteria:**

**Given** 网关处理请求
**When** 请求进入
**Then** 生成 UUID v7 Request ID 注入日志和响应头 `X-Request-ID`
**And** 日志记录 request_id/model/protocol/延迟/状态码

---

### Epic 2: 可观测性与运维增强
Prometheus metrics、OpenTelemetry 追踪、配置热加载。
**Out of Scope 来源:** Observability integration, Config hot-reload
**状态:** 🔜 待实现

#### Story 2.1: Prometheus Metrics 端点

As a 运维人员,
I want `/metrics` 端点暴露 Prometheus 格式指标,
So that 可通过 Grafana 等工具监控网关健康状态。

**Acceptance Criteria:**

**Given** 网关已启动
**When** 访问 `GET /metrics`
**Then** 返回 Prometheus 格式文本，包含请求总数、延迟分布（p50/p95/p99）、错误率
**And** 不影响正常请求处理性能

---

#### Story 2.2: OpenTelemetry 分布式追踪

As a 运维人员,
I want 请求经网关时生成 OpenTelemetry span,
So that 可追踪端到端请求链路。

**Acceptance Criteria:**

**Given** OTel  Collector 已配置
**When** 请求经过网关
**Then** 生成 trace span，包含 decode/routing/genai-call/encode 阶段
**And** span 携带 Request ID、model、protocol 标签

---

#### Story 2.3: 配置热加载

As a 平台工程师,
I want 配置文件变更自动重载路由规则,
So that 无需重启服务即可调整路由。

**Acceptance Criteria:**

**Given** 网关运行中
**When** config.yaml 被修改并保存
**Then** 路由规则自动重载，不中断进行中请求
**And** 新请求使用新规则，旧请求继续使用旧规则

---

### Epic 3: 管理能力增强
管理 REST API、Responses 会话管理高级特性。
**Out of Scope 来源:** Admin API/Dashboard, Responses 会话管理
**状态:** 🔜 待实现

#### Story 3.1: 管理 REST API

As a 平台工程师,
I want 管理 API 接口查询和修改运行时配置,
So that 可通过编程方式管理路由规则。

**Acceptance Criteria:**

**Given** 管理 API 已启用
**When** 查询当前路由规则
**Then** 返回 provider 列表和路由配置
**And** 支持动态添加/删除/修改路由规则

---

#### Story 3.2: Responses 会话管理增强

As a 开发者,
I want `previous_response_id` 和 `store` 等 Responses 高级特性被支持,
So that Responses 会话功能在跨协议场景下可用。

**Acceptance Criteria:**

**Given** 发送含 `previous_response_id` 的 Responses 请求
**When** 路由到非 Responses 后端
**Then** 会话状态在网关层维护
**And** 响应中包含正确的 `response_id`

---

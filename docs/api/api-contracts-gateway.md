# API 接口文档 — Gateway

> LLM Mux 网关对外暴露 3 个协议入口 + 1 个健康检查端点。
> 所有请求均为无状态 HTTP POST，响应为 JSON。

## 认证

| 协议 | 认证方式 |
|------|----------|
| OpenAI Chat | `Authorization: Bearer <key>` |
| Anthropic Messages | `x-api-key: <key>` |
| OpenAI Responses | `Authorization: Bearer <key>` 或 `x-api-key: <key>` |

API Key 白名单通过配置文件 `api_keys` 字段设置（可选，不配置则不验证）。

## 端点列表

### `GET /health`

健康检查。

**响应：** `200 OK`
```json
{"status": "ok"}
```

### `POST /v1/chat/completions`

OpenAI Chat Completions 兼容接口。

**入站协议：** `openai_chat`
**请求体：** 标准 OpenAI Chat Completion Request JSON
**响应体：** 标准 OpenAI Chat Completion Response JSON
**流式：** 支持（`stream: true`），SSE 格式

**关键字段映射：**
- `role: "system"` / `"developer"` → 提取为 system prompt
- `role: "assistant"` → 包含 `tool_calls` 解析
- `role: "tool"` → 关联 `tool_call_id`
- 多模态 `content: [{type: "image_url", ...}]` → 二进制内容

### `POST /v1/messages`

Anthropic Messages 兼容接口。

**入站协议：** `anthropic`
**请求体：** 标准 Anthropic Messages Request JSON
**响应体：** 标准 Anthropic Messages Response JSON
**流式：** 支持（SSE，`content_block_delta`/`message_stop` 事件）

**关键字段映射：**
- `system` 字段 → 提取为 system prompt
- `thinking` 配置 → 提取为 reasoning config
- 多模态 `content: [{type: "image", source: {...}}]` → 二进制内容
- SSE 流起始自动注入 `message_start` + `content_block_start` 事件

### `POST /v1/responses`

OpenAI Responses 兼容接口。

**入站协议：** `openai_responses`
**请求体：** 标准 OpenAI Responses Request JSON
**响应体：** 标准 OpenAI Responses Response JSON
**流式：** 支持（SSE，`response.output_text.delta`/`response.completed` 事件）

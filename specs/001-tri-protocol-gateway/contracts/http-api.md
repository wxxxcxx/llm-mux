# HTTP API 契约

**Feature**: LLM Mux 三协议互转网关 | **Date**: 2026-05-27

## 端点总览

| 方法 | 路径 | 用途 |
|------|------|------|
| `POST` | `/v1/chat/completions` | OpenAI Chat Completions 代理 |
| `POST` | `/v1/responses` | OpenAI Responses 代理 |
| `POST` | `/v1/messages` | Anthropic Messages 代理 |
| `GET` | `/health` | 健康检查 |

## 公共约定

- **Content-Type**: `application/json` (非流式), `text/event-stream` (流式)
- **认证**: `Authorization: Bearer <api_key>` 头
- **错误响应**: 均返回客户端协议对应的错误格式
- **流式**: 客户端请求体 `"stream": true` 时，响应为 SSE 格式
- **请求 ID**: 每个请求生成 UUID v7，通过 `Response Headers: X-Request-ID` 返回

---

## POST /v1/chat/completions

OpenAI Chat Completions API 兼容端点。

- **请求体**: OpenAI Chat Completions JSON（标准格式）
- **成功响应 (200)**: OpenAI Chat Completions JSON
- **流式响应 (200)**: `text/event-stream`，格式 `data: {chunk}\n\n`，结尾 `data: [DONE]\n\n`
- **认证失败 (401)**: OpenAI 错误格式 `{"error": {"message": "...", "type": "authentication_error"}}`
- **路由失败 (502)**: OpenAI 错误格式 `{"error": {"message": "...", "type": "server_error"}}`

---

## POST /v1/responses

OpenAI Responses API 兼容端点。

- **请求体**: OpenAI Responses JSON
- **成功响应 (200)**: OpenAI Responses JSON
- **流式响应 (200)**: SSE 格式
- **错误格式**: OpenAI Responses 错误格式

---

## POST /v1/messages

Anthropic Messages API 兼容端点。

- **请求体**: Anthropic Messages JSON
- **成功响应 (200)**: Anthropic Messages JSON
- **流式响应 (200)**: SSE 格式，Anthropic 事件类型（`message_start` / `content_block_start` / `content_block_delta` / `message_stop`）
- **认证失败 (401)**: Anthropic 错误格式 `{"type": "error", "error": {"type": "authentication_error", "message": "..."}}`

---

## GET /health

- **成功 (200)**: `{"status": "ok"}`
- **关闭中 (503)**: `{"status": "draining"}`
- 不要求认证

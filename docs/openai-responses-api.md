# OpenAI Responses API — Create Response

> 来源: `https://developers.openai.com/api/reference/resources/responses/methods/create`
> 端点: `POST /v1/responses`
> 提取时间: 2026年5月
> 注意: 这是 OpenAI 推荐的新一代统一 API

---

## 1. 概述

OpenAI Responses API 是新一代统一 API, 用于生成模型响应。相比 Chat Completions, 它提供了更丰富的功能:
- **有状态会话管理**: `conversation` 和 `previous_response_id` 自动追踪对话历史
- **多态 InputItem**: 消息、工具调用、工具结果、推理等统一为 item 类型
- **内置工具生态**: web_search, file_search, code_interpreter, image_generation, computer use, MCP 等
- **后台执行**: `background` 模式
- **上下文压缩**: `context_management` 自动管理超长对话
- **Prompt 模板**: `prompt` 参数引用可复用模板

---

## 2. 请求体参数

### 2.1 必填参数

#### `model` — string (enum)

对标准模型和 Responses-only 模型的支持:

| 类别 | 示例 |
|---|---|
| 标准模型 | `gpt-5.4`, `gpt-5.1`, `gpt-4.1`, `o4-mini`, `o3` |
| Responses-only | `o1-pro`, `o3-pro`, `o3-deep-research`, `computer-use-preview`, `gpt-5-codex`, `gpt-5-pro`, `gpt-5.1-codex-max` |

#### `input` — string | array of InputItem

文本、图像或文件输入。
- `string`: 等价于单条 `role: "user"` 消息
- `array`: InputItem 列表（见 §3）

---

### 2.2 可选参数

#### `instructions` — string | null (默认 null)

系统/开发者指令, 插入到模型上下文中。
- 使用 `previous_response_id` 时, **不会**自动携带上一轮的 instructions

#### `max_output_tokens` — integer | null (默认 null, min 16)

生成上限（包括可见输出 + 推理 token）。

#### `temperature` — number | null (默认 1, 范围 0–2)

采样温度。推荐与 `top_p` 二选一修改。

#### `top_p` — number | null (默认 1, 范围 0–1)

核采样。

#### `top_logprobs` — integer (0–20)

每个位置返回的最可能 token 数及其对数概率。

#### `reasoning` — object | null

Reasoning 模型配置 (gpt-5 / o-series):

```json
{
  "effort": "none" | "minimal" | "low" | "medium" | "high" | "xhigh",
  "summary": "auto" | "concise" | "detailed"
}
```

**`effort` 默认值:**
| 模型 | 默认值 | 说明 |
|---|---|---|
| gpt-5.1 | `"none"` | 支持 none/low/medium/high |
| gpt-5.1 之前 | `"medium"` | 不支持 `"none"` |
| gpt-5-pro | `"high"` | 仅支持 high |

**`summary`** — 推理摘要:
- `"auto"`: 自动决定
- `"concise"`: 简洁
- `"detailed"`: 详细

> `generate_summary` 已弃用, 使用 `summary` 替代

#### `text` — object

文本响应配置:

```json
{
  "format": { /* 见下方 */ },
  "verbosity": "low" | "medium" | "high"
}
```

**`text.format` (响应格式) — one of:**

| 格式 | `type` | 说明 |
|---|---|---|
| `ResponseFormatText` | `"text"` | 默认, 纯文本 |
| `TextResponseFormatJsonSchema` | `"json_schema"` | Structured Outputs |
| `ResponseFormatJsonObject` | `"json_object"` | 旧 JSON 模式 |
| `ResponseFormatTextGrammar` | `"grammar"` | 自定义语法 |

**JSON Schema format:**
```json
{
  "type": "json_schema",
  "name": "weather_response",
  "description": "Weather data response",
  "schema": { /* JSON Schema */ },
  "strict": true
}
```

**Grammar format:**
```json
{
  "type": "grammar",
  "grammar": "<custom-grammar-definition>"
}
```

#### `tools` — array of Tool (默认 `[]`)

工具定义（见 §4）。

#### `tool_choice` — oneOf (默认 `"auto"`)

工具选择控制（见 §5）。

#### `parallel_tool_calls` — boolean | null (默认 `true`)

是否允许并行工具调用。

#### `max_tool_calls` — integer | null (默认 null)

整个响应的内置工具最大调用总数（非每个工具）。超出上限的调用被忽略。

#### `truncation` — `"auto"` | `"disabled"` | null (默认 `"disabled"`)

- `"auto"`: 输入超过上下文窗口时, 丢弃开头部分
- `"disabled"`: 超出上下文窗口时返回 400 错误

#### `previous_response_id` — string | null

多轮对话中上一轮响应的 ID（无状态模式）。不能与 `conversation` 同时使用。

#### `conversation` — string | `{id: "conv_123"}` | null

会话 ID 或对象。会话中的 items 会预置到 input 前; 完成后 input/output items 自动加入会话。不能与 `previous_response_id` 同时使用。

#### `context_management` — array of ContextManagementParam | null

上下文管理, 最小 1 项:

```json
[
  {
    "type": "compaction",
    "compact_threshold": 2000  // token 阈值, min 1000
  }
]
```

目前仅支持 `"compaction"` 类型。

#### `store` — boolean | null (默认 `true`)

是否存储响应用于后续 API 检索。

#### `stream` — boolean | null (默认 `false`)

是否通过 SSE 流式传输。

#### `stream_options` — object | null

仅 `stream: true` 时使用:
```json
{
  "include_obfuscation": true  // 随机字符混淆 delta 事件, 缓解旁路攻击
}
```

#### `background` — boolean | null (默认 `false`)

是否在后台运行。

#### `prompt` — object | null

Prompt 模板引用:

```json
{
  "id": "prompt_123",
  "version": "v2",
  "variables": "some value"  // 或对象 { varname: { type: "input_text", text: "..." } }
}
```

#### `safety_identifier` — string (最大 64 字符)

帮助检测政策违规的用户稳定标识符。建议哈希用户名/邮箱。

#### `prompt_cache_key` — string

用于优化相似请求的缓存命中率。替代已弃用的 `user`。

#### `prompt_cache_retention` — `"24h"` | `"in_memory"` | null

缓存前缀的存储时长:
- `"in_memory"`: 仅内存
- `"24h"`: 最长 24 小时

#### `user` — string — DEPRECATED

已弃用。使用 `prompt_cache_key` 替代。

#### `service_tier` — `"auto"` | `"default"` | `"flex"` | `"scale"` | `"priority"` | null (默认 `"auto"`)

处理类型。响应正文返回实际使用的层级（可能与请求不同）。

#### `metadata` — object (map) | null (默认 null)

最多 16 对键值。键最大 64 字符, 值最大 512 字符。

#### `include` — array of IncludeEnum | null

请求在响应中包含额外输出数据:

| 值 | 说明 |
|---|---|
| `file_search_call.results` | 文件搜索工具的结果 |
| `web_search_call.results` | Web 搜索工具的结果 |
| `web_search_call.action.sources` | Web 搜索工具的来源 |
| `message.input_image.image_url` | 输入消息中的图片 URL |
| `computer_call_output.output.image_url` | computer call 输出的图片 URL |
| `code_interpreter_call.outputs` | 代码解释器 Python 执行的输出 |
| `reasoning.encrypted_content` | 加密推理 token（无状态多轮、store=false 或零数据保留） |
| `message.output_text.logprobs` | assistant 消息的 logprobs |

---

## 3. InputItem 类型

`input` 参数为数组时, 每个元素为以下类型之一（按 `type` 区分）：

### 3.1 消息输入

#### EasyInputMessage (`type: "message"`)
简化格式:

```json
{
  "type": "message",
  "role": "user" | "assistant" | "system" | "developer",
  "content": "string" | InputMessageContentList,
  "phase": null  // 可选 stage
}
```

#### InputMessage (完整格式)
```json
{
  "type": "message",
  "role": "user" | "system" | "developer",
  "status": "in_progress" | "completed" | "incomplete",
  "content": [/* InputContent 数组 */]
}
```

#### OutputMessage
Assistant 消息输出（响应中返回, 也可作为输入传递）:
```json
{
  "type": "output_message",
  "status": "completed",
  "content": [/* content blocks */]
}
```

### 3.2 消息内容类型 (InputContent)

消息 `content` 数组的元素:

```json
// 文本
{"type": "input_text", "text": "What's the weather?"}

// 图片
{
  "type": "input_image",
  "image_url": "https://example.com/photo.jpg",
  "file_id": "file-123",
  "detail": "high" | "low" | "auto" | "original"
}

// 文件
{
  "type": "input_file",
  "file_id": "file-456",
  "filename": "report.pdf",
  "file_data": "<content>",
  "file_url": "https://example.com/doc.pdf"
}
```

### 3.3 工具调用输入

| `type` 值 | 说明 |
|---|---|
| `file_search_call` | 文件搜索工具调用 |
| `web_search_call` | Web 搜索工具调用 |
| `function_call` | 函数工具调用 |
| `code_interpreter_call` | 代码解释器工具调用 |
| `computer_call` | 计算机工具调用 |
| `local_shell_call` | 本地 Shell 工具调用 |
| `image_generation_call` | 图像生成工具调用 |
| `mcp_call` | MCP 工具调用 |
| `mcp_list_tools` | MCP 列出工具 |
| `mcp_approval_request` | MCP 审批请求 |
| `mcp_approval_response` | MCP 审批响应 |
| `custom_tool_call` | 自定义工具调用 |
| `tool_search_call` | 工具搜索调用 |
| `apply_patch_call` | Apply Patch 工具调用 |
| `function_shell_call` | Function Shell 工具调用 |

### 3.4 工具结果输入

| `type` 值 | 说明 |
|---|---|
| `function_call_output` | 函数调用输出 |
| `computer_call_output` | Computer call 输出 |
| `local_shell_call_output` | 本地 Shell 输出 |
| `custom_tool_call_output` | 自定义工具输出 |
| `tool_search_output` | 工具搜索输出 |
| `apply_patch_call_output` | Apply Patch 输出 |
| `function_shell_call_output` | Function Shell 输出 |

**function_call_output 结构:**
```json
{
  "type": "function_call_output",
  "call_id": "call_123",
  "output": "72°F, sunny" | [{ type: "input_text", text: "..." }],
  "status": "completed"
}
```

### 3.5 特殊输入

| `type` 值 | 说明 |
|---|---|
| `reasoning` | Chain of thought reasoning（在手动管理上下文的后续 turn 中使用） |
| `compaction_summary` | 压缩摘要 |
| `item_reference` | 引用其他 item（`{type: "item_reference", id: "item_abc"}`） |

---

## 4. 工具类型 (`tools` 数组)

### 4.1 Function Tool

```json
{
  "type": "function",
  "name": "get_weather",
  "description": "Get the current weather",
  "parameters": { /* JSON Schema */ },
  "strict": true,
  "defer_loading": false
}
```

### 4.2 File Search

```json
{
  "type": "file_search",
  "vector_store_ids": ["vs_123", "vs_456"],
  "max_num_results": 20,  // 1-50
  "ranking_options": { /* ... */ },
  "filters": null
}
```

### 4.3 Web Search

```json
{
  "type": "web_search",
  "filters": {
    "allowed_domains": ["docs.openai.com"]
  },
  "user_location": {
    "type": "approximate",
    "city": "San Francisco",
    "country": "US",
    "region": "California",
    "timezone": "America/Los_Angeles"
  },
  "search_context_size": "medium"
}
```

也支持类型 `"web_search_2025_08_26"`。

### 4.4 Web Search Preview

```json
{
  "type": "web_search_preview",
  "user_location": { /* ApproximateLocation */ },
  "search_context_size": "medium",
  "search_content_types": [/* SearchContentType */]
}
```

也支持类型 `"web_search_preview_2025_03_11"`。

### 4.5 Code Interpreter

```json
{
  "type": "code_interpreter",
  "container": {
    "type": "auto",
    "file_ids": ["file-1", "file-2"],
    "memory_limit": "4g"
  }
}
```

`memory_limit`: `"1g"` | `"4g"` | `"16g"` | `"64g"`。

也可使用显式 `container: "container_123"`。

### 4.6 Computer Tool

```json
{
  "type": "computer"
}
```

### 4.7 Computer Use Preview

```json
{
  "type": "computer_use_preview",
  "environment": "windows" | "mac" | "linux" | "ubuntu" | "browser",
  "display_width": 1280,
  "display_height": 720
}
```

### 4.8 Image Generation

```json
{
  "type": "image_generation",
  "model": "gpt-image-1",
  "quality": "high",
  "size": "1024x1024"
}
```

`model`: `"gpt-image-1"` | `"gpt-image-1-mini"` | `"gpt-image-1.5"`（默认 `gpt-image-1`）。
`quality`: `"low"` | `"medium"` | `"high"` | `"auto"`（默认 `"auto"`）。

### 4.9 MCP Tool

```json
{
  "type": "mcp",
  "server_label": "gmail",
  "server_url": "https://...",  // 或使用 connector_id
  "authorization": "Bearer token",
  "server_description": "optional"
}
```

**Connector IDs:**
| ID | 说明 |
|---|---|
| `connector_dropbox` | Dropbox |
| `connector_gmail` | Gmail |
| `connector_googlecalendar` | Google Calendar |
| `connector_googledrive` | Google Drive |
| `connector_microsoftteams` | Microsoft Teams |
| `connector_outlookcalendar` | Outlook Calendar |
| `connector_outlookemail` | Outlook Email |
| `connector_sharepoint` | SharePoint |

*`server_url` 或 `connector_id` 二选一必填*

### 4.10 其他工具

| Tool | `type` | 说明 |
|---|---|---|
| Local Shell | — | 本地 Shell 执行 |
| Function Shell | — | 函数 Shell |
| Custom Tool | — | 自定义工具定义 |
| Namespace Tool | — | 命名空间工具 |
| Tool Search | — | 工具搜索 |
| Apply Patch | `"apply_patch"` | 使用 unified diff 创建/删除/更新文件 |

---

## 5. Tool Choice (`tool_choice`)

### 5.1 字符串简写

| 值 | 说明 |
|---|---|
| `"none"` | 不调用任何工具, 生成消息 |
| `"auto"` | 模型可自行选择 |
| `"required"` | 必须调用一个或多个工具 |

### 5.2 Allowed Tools (对象)

```json
{
  "type": "allowed_tools",
  "mode": "auto" | "required",
  "tools": [
    {"type": "function", "name": "get_weather"},
    {"type": "mcp", "server_label": "deepwiki"},
    {"type": "image_generation"}
  ]
}
```

### 5.3 特定工具强制调用

```json
// 函数
{"type": "function", "name": "get_weather"}

// MCP 工具
{"type": "mcp", "server_label": "...", "tool_name": "..."}

// 自定义工具
{"type": "custom", "name": "my_tool"}

// Apply Patch
{"type": "apply_patch"}

// Function Shell
{"type": "function_shell"}
```

---

## 6. 全部请求参数速查

| # | 参数 | 类型 | 必填 | 默认 | 说明 |
|---|---|---|---|---|---|
| 1 | `model` | string | **是** | — | 模型 ID |
| 2 | `input` | string \| array | **是** | — | 输入 |
| 3 | `instructions` | string \| null | 否 | null | 系统提示 |
| 4 | `max_output_tokens` | integer \| null | 否 | null | 最大输出 |
| 5 | `temperature` | number \| null | 否 | 1 | 温度 |
| 6 | `top_p` | number \| null | 否 | 1 | 核采样 |
| 7 | `top_logprobs` | integer | 否 | — | logprobs |
| 8 | `reasoning` | object \| null | 否 | null | 推理配置 |
| 9 | `text` | object | 否 | — | 文本格式/verbosity |
| 10 | `tools` | array | 否 | `[]` | 工具定义 |
| 11 | `tool_choice` | oneOf | 否 | `"auto"` | 工具选择 |
| 12 | `parallel_tool_calls` | boolean \| null | 否 | true | 并行工具调用 |
| 13 | `max_tool_calls` | integer \| null | 否 | null | 最大工具调用数 |
| 14 | `truncation` | `"auto"` \| `"disabled"` \| null | 否 | `"disabled"` | 截断策略 |
| 15 | `previous_response_id` | string \| null | 否 | null | 前一轮响应 ID |
| 16 | `conversation` | string \| object \| null | 否 | null | 会话 |
| 17 | `context_management` | array \| null | 否 | null | 上下文管理 |
| 18 | `store` | boolean \| null | 否 | true | 存储响应 |
| 19 | `stream` | boolean \| null | 否 | false | 流式 |
| 20 | `stream_options` | object \| null | 否 | null | 流式选项 |
| 21 | `background` | boolean \| null | 否 | false | 后台模式 |
| 22 | `prompt` | object \| null | 否 | null | Prompt 模板 |
| 23 | `safety_identifier` | string | 否 | — | 安全标识 |
| 24 | `prompt_cache_key` | string | 否 | — | 缓存键 |
| 25 | `prompt_cache_retention` | `"24h"` \| `"in_memory"` \| null | 否 | — | 缓存保留 |
| 26 | `user` | string | 否 | — | **已弃用** |
| 27 | `service_tier` | string \| null | 否 | `"auto"` | 服务等级 |
| 28 | `metadata` | object \| null | 否 | null | 元数据 |
| 29 | `include` | array \| null | 否 | null | 额外输出数据 |

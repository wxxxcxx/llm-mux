# Anthropic Messages API — Create Message

> 来源: `https://platform.claude.com/docs/en/api/messages/create`
> 端点: `POST /v1/messages`
> 提取时间: 2026年5月

---

## 1. 概述

发送包含文本和/或图像内容的结构化输入消息列表, 模型将生成对话中的下一条消息。

Messages API 可用于单次查询或**无状态多轮对话**。

- 消息角色: `user`、`assistant`（无 `system` 角色, 通过顶层 `system` 参数设置）
- 消息限流: 单次请求最多 100,000 条消息
- 连续相同角色会自动合并为一个 turn

---

## 2. 请求体参数

### 2.1 必填参数

#### `model` — string (enum)

| 值 | 说明 |
|---|---|
| `claude-opus-4-7` | Frontier intelligence for long-running agents and coding |
| `claude-mythos-preview` | New class of intelligence, strongest in coding and cybersecurity |
| `claude-opus-4-6` | Frontier intelligence for long-running agents and coding |
| `claude-sonnet-4-6` | Best combination of speed and intelligence |
| `claude-haiku-4-5` | Fastest model with near-frontier intelligence |
| `claude-haiku-4-5-20251001` | Fastest model with near-frontier intelligence |
| `claude-opus-4-5` | Premium model combining maximum intelligence with practical performance |
| `claude-opus-4-5-20251101` | Premium model combining maximum intelligence with practical performance |
| `claude-sonnet-4-5` | High-performance model for agents and coding |
| `claude-sonnet-4-5-20250929` | High-performance model for agents and coding |
| `claude-opus-4-1` | Exceptional model for specialized complex tasks |
| `claude-opus-4-1-20250805` | Exceptional model for specialized complex tasks |
| `claude-opus-4-0` | Powerful model for complex tasks |
| `claude-opus-4-20250514` | Powerful model for complex tasks |
| `claude-sonnet-4-0` | High-performance model with extended thinking |
| `claude-sonnet-4-20250514` | High-performance model with extended thinking |
| `claude-3-haiku-20240307` | Fast and cost-effective model |

也接受任意自定义 `string`。

#### `messages` — array of MessageParam

每个元素:

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `role` | `"user"` \| `"assistant"` | 是 | 消息角色 |
| `content` | `string` \| `array of ContentBlockParam` | 是 | 消息内容 |

当 `content` 为 `string` 时, 等价于 `[{"type": "text", "text": "<value>"}]`。

示例:
```json
{"role": "user", "content": "Hello, Claude"}
```

```json
{"role": "user", "content": [{"type": "text", "text": "Hello, Claude"}]}
```

如果最后一条消息使用 `assistant` 角色, 响应内容将从该消息的内容之后继续（用于预填充/约束模型输出）:

```json
[
  {"role": "user", "content": "What's the Greek name for Sun? (A) Sol (B) Helios (C) Sun"},
  {"role": "assistant", "content": "The best answer is ("}
]
```

#### `max_tokens` — number (必填)

最大生成 token 数。模型可能在此之前停止。

- 设为 `0` 可填充 prompt cache 但不生成响应
- 不同模型有不同最大值

---

### 2.2 可选参数

#### `system` — string | array of TextBlockParam

系统提示, 用于提供上下文和指令。

```json
"system": "You are a helpful assistant."
// 或
"system": [
  {"type": "text", "text": "You are a helpful assistant.", "cache_control": {"type": "ephemeral"}}
]
```

TextBlock 字段: `text` (string), `type: "text"`, 可选 `cache_control`, `citations`。

#### `thinking` — ThinkingConfigParam

扩展思考配置, 启用后响应包含 `thinking` 内容块。

**三种模式:**

| 模式 | `type` | 字段 |
|---|---|---|
| **Enabled** | `"enabled"` | `budget_tokens` (number, >=1024, <max_tokens), `display` (\`"summarized"\` \| \`"omitted"\`, 默认 summarized) |
| **Disabled** | `"disabled"` | 无额外参数 |
| **Adaptive** | `"adaptive"` | `display` (\`"summarized"\` \| \`"omitted"\`, 默认 summarized) |

- `display: "omitted"` — thinking 内容被省略, 但返回签名保证多轮连续性
- `display: "summarized"` — 返回完整 thinking

#### `temperature` — number (默认 1.0, 范围 0.0–1.0)

随机性控制:
- 接近 0.0: 分析型、可选择
- 接近 1.0: 创造型、生成型

注意: 即使 `temperature=0.0`, 结果也不完全确定。

#### `top_k` — number

每步仅从 Top K 选项中采样。高级用途。

#### `top_p` — number

核采样 (nucleus sampling) — 累计概率截断。高级用途。

#### `stop_sequences` — array of string

自定义停止序列。模型遇到匹配序列时, `stop_reason` = `"stop_sequence"`。

#### `tools` — array of ToolUnion

工具定义, 模型可能返回 `tool_use` 内容块。分两类:
- **客户端工具**: 用户自定义
- **服务端工具**: 内置（web_search, code_execution 等）

详见 §4。

#### `tool_choice` — ToolChoice

控制模型如何使用工具:

| 变体 | `type` | 额外字段 | 说明 |
|---|---|---|---|
| `ToolChoiceAuto` | `"auto"` | `disable_parallel_tool_use` (可选, 默认 false) | 模型自动决定是否使用工具 |
| `ToolChoiceAny` | `"any"` | `disable_parallel_tool_use` (可选, 默认 false) | 模型必须使用可用工具 |
| `ToolChoiceTool` | `"tool"` | `name` (string), `disable_parallel_tool_use` | 强制使用指定工具 |
| `ToolChoiceNone` | `"none"` | — | 禁止使用工具 |

#### `stream` — boolean

是否通过 SSE 增量流式传输响应。

#### `output_config` — OutputConfig

输出配置:

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `effort` | `"low"` \| `"medium"` \| `"high"` \| `"xhigh"` \| `"max"` | 否 | 输出努力级别 |
| `format` | `JSONOutputFormat` | 否 | 结构化输出格式 |

**JSONOutputFormat:**
```json
{
  "type": "json_schema",
  "schema": { /* JSON Schema */ }
}
```

#### `cache_control` — CacheControlEphemeral

顶层缓存控制, 自动将 cache_control 标记应用于请求中**最后一个可缓存块**。

```json
{
  "type": "ephemeral",
  "ttl": "5m"  // 或 "1h", 默认 5m
}
```

#### `container` — string

容器标识, 跨请求复用。将文件上传到容器使用 `ContainerUploadBlockParam`。

#### `inference_geo` — string

指定推理处理的地理区域。未指定时使用工作空间默认值。

#### `service_tier` — `"auto"` | `"standard_only"`

优先容量 vs 标准容量。

#### `metadata` — Metadata

```json
{
  "user_id": "string"
}
```

- `user_id`: 外部用户标识（UUID/hash/不透明标识, 不含 PII）

---

## 3. 内容块类型 (ContentBlockParam)

### 3.1 TextBlockParam (`type: "text"`)

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `text` | string | 是 | 文本内容 |
| `type` | `"text"` | 是 | 固定值 |
| `cache_control` | CacheControlEphemeral | 否 | 缓存控制断点 |
| `citations` | array of TextCitationParam | 否 | 引用信息 |

**TextCitationParam 五种变体:**

| 变体 | `type` | 说明 |
|---|---|---|
| CitationCharLocationParam | `"char_location"` | 基于字符位置的引用 |
| CitationPageLocationParam | `"page_location"` | 基于页码的引用 |
| CitationContentBlockLocationParam | `"content_block_location"` | 基于内容块的引用 |
| CitationWebSearchResultLocationParam | `"web_search_result_location"` | Web 搜索结果引用 |
| CitationSearchResultLocationParam | `"search_result_location"` | 搜索工具结果引用 |

### 3.2 ImageBlockParam (`type: "image"`)

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `source` | Base64ImageSource \| URLImageSource | 是 | 图像源 |
| `type` | `"image"` | 是 | 固定值 |
| `cache_control` | CacheControlEphemeral | 否 | 缓存控制断点 |

**Base64ImageSource:**
```json
{
  "type": "base64",
  "media_type": "image/jpeg" | "image/png" | "image/gif" | "image/webp",
  "data": "<base64-encoded>"
}
```

**URLImageSource:**
```json
{
  "type": "url",
  "url": "https://example.com/image.jpg"
}
```

### 3.3 DocumentBlockParam (`type: "document"`)

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `source` | Base64PDFSource \| PlainTextSource \| ContentBlockSource \| URLPDFSource | 是 | 文档源 |
| `type` | `"document"` | 是 | 固定值 |
| `cache_control` | CacheControlEphemeral | 否 | |
| `citations` | `{enabled: boolean}` | 否 | 引用配置 |
| `context` | string | 否 | 上下文 |
| `title` | string | 否 | 文档标题 |

**文档源类型:**

| 源类型 | `type` | 说明 |
|---|---|---|
| Base64PDFSource | `"base64"` | `data`, `media_type: "application/pdf"` |
| PlainTextSource | `"text"` | `data`, `media_type: "text/plain"` |
| ContentBlockSource | `"content"` | `content: string \| array of (TextBlockParam \| ImageBlockParam)` |
| URLPDFSource | `"url"` | `url: string` |

### 3.4 SearchResultBlockParam (`type: "search_result"`)

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content` | array of TextBlockParam | 是 | 文本内容 |
| `source` | string | 是 | 来源标识 |
| `title` | string | 是 | 结果标题 |
| `type` | `"search_result"` | 是 | 固定值 |
| `cache_control` | CacheControlEphemeral | 否 | |
| `citations` | CitationsConfigParam | 否 | |

### 3.5 ThinkingBlockParam (`type: "thinking"`)

表示 Claude 的推理过程。在多轮对话中传递先前的 thinking。

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `thinking` | string | 是 | thinking 内容 |
| `signature` | string | 是 | 连续性签名 |
| `type` | `"thinking"` | 是 | 固定值 |

### 3.6 RedactedThinkingBlockParam (`type: "redacted_thinking"`)

当 `thinking.display: "omitted"` 时的截断 thinking 内容。

| 字段 | 类型 | 必填 |
|---|---|---|
| `data` | string | 是 |
| `type` | `"redacted_thinking"` | 是 |

### 3.7 ToolUseBlockParam (`type: "tool_use"`)

模型生成的工具使用请求（在后续对话中传回）。

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `id` | string | 是 | 工具调用 ID |
| `name` | string | 是 | 工具名称 |
| `input` | object (map) | 是 | 输入参数 |
| `type` | `"tool_use"` | 是 | 固定值 |
| `cache_control` | CacheControlEphemeral | 否 | |
| `caller` | DirectCaller \| ServerToolCaller \| ServerToolCaller20260120 | 否 | 调用来源 |

**caller 变体:**
- `DirectCaller`: `{type: "direct"}` — 直接调用
- `ServerToolCaller`: `{type: "code_execution_20250825", tool_id: string}` — 服务端工具调用
- `ServerToolCaller20260120`: `{type: "code_execution_20260120", tool_id: string}` — 新版服务端工具调用

### 3.8 ToolResultBlockParam (`type: "tool_result"`)

工具执行结果, 传回模型。

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `tool_use_id` | string | 是 | 对应 `tool_use` 的 id |
| `type` | `"tool_result"` | 是 | 固定值 |
| `content` | string \| array of (TextBlockParam \| ImageBlockParam \| SearchResultBlockParam \| DocumentBlockParam \| ToolReferenceBlockParam) | 否 | 结果内容 |
| `is_error` | boolean | 否 | 是否为错误结果 |
| `cache_control` | CacheControlEphemeral | 否 | |

### 3.9 ServerToolUseBlockParam (`type: "server_tool_use"`)

服务端工具使用块。

| `name` 值 | 说明 |
|---|---|
| `web_search` | Web 搜索 |
| `web_fetch` | Web 获取 |
| `code_execution` | 代码执行 |
| `bash_code_execution` | Bash 执行 |
| `text_editor_code_execution` | 文本编辑器执行 |
| `tool_search_tool_regex` | 正则工具搜索 |
| `tool_search_tool_bm25` | BM25 工具搜索 |

### 3.10 WebSearchToolResultBlockParam (`type: "web_search_tool_result"`)

| 字段 | 说明 |
|---|---|
| `content` | WebSearchResultBlockParam []（结果）或 WebSearchToolRequestError（错误） |
| `tool_use_id` | 对应 ID |

**WebSearchResultBlockParam:**
```json
{
  "encrypted_content": "string",
  "title": "string",
  "type": "web_search_result",
  "url": "string",
  "page_age": "string (optional)"
}
```

**错误码:** `invalid_tool_input`, `unavailable`, `max_uses_exceeded`, `too_many_requests`, `query_too_long`, `request_too_large`

### 3.11 WebFetchToolResultBlockParam (`type: "web_fetch_tool_result"`)

成功:
```json
{
  "type": "web_fetch_result",
  "url": "string",
  "content": { DocumentBlockParam },
  "retrieved_at": "ISO 8601 (optional)"
}
```

错误码: `invalid_tool_input`, `url_too_long`, `url_not_allowed`, `url_not_accessible`, `unsupported_content_type`, `too_many_requests`, `max_uses_exceeded`, `unavailable`

### 3.12 CodeExecutionToolResultBlockParam (`type: "code_execution_tool_result"`)

**成功结果:**
```json
{
  "type": "code_execution_result",
  "stdout": "string",
  "stderr": "string",
  "return_code": 0,
  "content": [{"type": "code_execution_output", "file_id": "string"}]
}
```

**加密输出 (PFC + web_search):**
```json
{
  "type": "encrypted_code_execution_result",
  "encrypted_stdout": "string",
  "stderr": "string",
  "return_code": 0,
  "content": [...]
}
```

错误码: `invalid_tool_input`, `unavailable`, `too_many_requests`, `execution_time_exceeded`

### 3.13 BashCodeExecutionToolResultBlockParam (`type: "bash_code_execution_tool_result"`)

类似 code_execution, 差异:
- 成功: `type: "bash_code_execution_result"`, content 含 `BashCodeExecutionOutputBlockParam`
- 错误额外: `output_file_too_large`

### 3.14 TextEditorCodeExecutionToolResultBlockParam (`type: "text_editor_code_execution_tool_result"`)

四种结果:
- **Error**: `type: "text_editor_code_execution_tool_result_error"`, `error_code` + `error_message`
- **View**: `type: "text_editor_code_execution_view_result"`, `content`, `file_type` (text/image/pdf), `num_lines`, `start_line`, `total_lines`
- **Create**: `type: "text_editor_code_execution_create_result"`, `is_file_update`
- **StrReplace**: `type: "text_editor_code_execution_str_replace_result"`, `lines`, `new_lines`, `new_start`, `old_lines`, `old_start`

### 3.15 ToolSearchToolResultBlockParam (`type: "tool_search_tool_result"`)

成功: `tool_references: array of {tool_name, type: "tool_reference"}`

### 3.16 ContainerUploadBlockParam (`type: "container_upload"`)

```json
{
  "type": "container_upload",
  "file_id": "string",
  "cache_control": { /* optional */ }
}
```

---

## 4. 工具定义 (tools 数组)

### 4.1 用户自定义工具 (Tool)

```json
{
  "name": "get_stock_price",
  "description": "Get the current stock price for a given ticker symbol.",
  "input_schema": {
    "type": "object",
    "properties": {
      "ticker": {
        "type": "string",
        "description": "The stock ticker symbol, e.g. AAPL for Apple Inc."
      }
    },
    "required": ["ticker"]
  }
}
```

完整字段:

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | 是 | 工具名称 |
| `input_schema` | object | 是 | JSON Schema (draft 2020-12) |
| `input_schema.type` | `"object"` | 是 | Schema 类型 |
| `input_schema.properties` | map | 否 | 属性定义 |
| `input_schema.required` | array of string | 否 | 必需属性 |
| `description` | string | 否 | 工具描述（强烈推荐） |
| `type` | `"custom"` | 否 | 工具变体类型 |
| `allowed_callers` | array of `"direct"` \| `"code_execution_20250825"` \| `"code_execution_20260120"` | 否 | 调用者限制 |
| `cache_control` | CacheControlEphemeral | 否 | 缓存控制 |
| `defer_loading` | boolean | 否 | 延迟加载, 仅通过 tool_search 返回的 tool_reference 加载 |
| `eager_input_streaming` | boolean | 否 | 启用快速输入流 |
| `input_examples` | array of map | 否 | 输入示例 |
| `strict` | boolean | 否 | 严格 schema 校验 |

### 4.2 服务端内置工具

#### Bash (`ToolBash20250124`)
```json
{"name": "bash", "type": "bash_20250124"}
```

#### Code Execution (3 个版本)
```json
{"name": "code_execution", "type": "code_execution_20250522"}
{"name": "code_execution", "type": "code_execution_20250825"}
{"name": "code_execution", "type": "code_execution_20260120"}  // REPL state persistence, daemon mode + gVisor checkpoint
```

#### Memory (`MemoryTool20250818`)
```json
{"name": "memory", "type": "memory_20250818"}
```

#### Text Editor (3 个版本)
```json
{"name": "str_replace_editor", "type": "text_editor_20250124"}
{"name": "str_replace_based_edit_tool", "type": "text_editor_20250429"}
{"name": "str_replace_based_edit_tool", "type": "text_editor_20250728"}  // 额外: max_characters
```

#### Web Search (2 个版本)
```json
{
  "name": "web_search",
  "type": "web_search_20250305",  // 或 web_search_20260209
  "allowed_domains": ["example.com"],
  "blocked_domains": ["bad-site.com"],
  "max_uses": 5,
  "user_location": {
    "type": "approximate",
    "city": "San Francisco",
    "country": "US",
    "region": "California",
    "timezone": "America/Los_Angeles"
  }
}
```

#### Web Fetch (3 个版本)
```json
{
  "name": "web_fetch",
  "type": "web_fetch_20250910",  // 或 web_fetch_20260209, web_fetch_20260309
  "allowed_domains": ["docs.anthropic.com"],
  "blocked_domains": [],
  "citations": {"enabled": true},
  "max_content_tokens": 10000,
  "max_uses": 10,
  "use_cache": false  // 仅 web_fetch_20260309
}
```

#### Tool Search (2 个版本)
```json
{"name": "tool_search_tool_bm25", "type": "tool_search_tool_bm25_20251119"}
{"name": "tool_search_tool_regex", "type": "tool_search_tool_regex_20251119"}
```

---

## 5. 响应示例

请求:
```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "messages": [
    {"role": "user", "content": "Hello, Claude"}
  ]
}
```

基本响应:
```json
{
  "id": "msg_01XFDJDYqA...",
  "type": "message",
  "role": "assistant",
  "model": "claude-sonnet-4-5-20250929",
  "content": [
    {"type": "text", "text": "Hello! How can I assist you today?"}
  ],
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  }
}
```

`stop_reason` 可能值: `"end_turn"`, `"max_tokens"`, `"stop_sequence"`, `"tool_use"`

工具调用响应:
```json
{
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01D7FLrfh4GYq7yT1ULFeyMV",
      "name": "get_stock_price",
      "input": {"ticker": "^GSPC"}
    }
  ],
  "stop_reason": "tool_use"
}
```

Thinking 响应:
```json
{
  "content": [
    {"type": "thinking", "thinking": "Let me analyze this...", "signature": "..."},
    {"type": "text", "text": "Here is my analysis..."}
  ]
}
```

---

## 6. 全部请求参数速查

| # | 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| 1 | `model` | string (enum) | **是** | 模型标识 |
| 2 | `messages` | array of MessageParam | **是** | 对话消息 |
| 3 | `max_tokens` | number | **是** | 最大输出 token (0=仅缓存) |
| 4 | `system` | string \| array of TextBlockParam | 否 | 系统提示 |
| 5 | `stop_sequences` | array of string | 否 | 停止序列 |
| 6 | `temperature` | number (0–1) | 否 | 随机性 |
| 7 | `top_k` | number | 否 | Top-K 采样 |
| 8 | `top_p` | number | 否 | 核采样 |
| 9 | `thinking` | ThinkingConfigParam | 否 | 扩展思考 |
| 10 | `tool_choice` | ToolChoice | 否 | 工具使用控制 |
| 11 | `tools` | array of ToolUnion | 否 | 工具定义 |
| 12 | `stream` | boolean | 否 | 流式 |
| 13 | `metadata` | Metadata | 否 | 用户 ID |
| 14 | `cache_control` | CacheControlEphemeral | 否 | 缓存控制 |
| 15 | `container` | string | 否 | 容器 ID |
| 16 | `inference_geo` | string | 否 | 推理区域 |
| 17 | `service_tier` | `"auto"` \| `"standard_only"` | 否 | 服务等级 |
| 18 | `output_config` | OutputConfig | 否 | 输出格式/努力 |

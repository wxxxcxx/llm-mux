# OpenAI Chat Completions API — Create Chat Completion

> 来源: `https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create`
> 端点: `POST /chat/completions`
> 提取时间: 2026年5月
> 注意: 部分参数已被 OpenAI 标记为"已弃用", 这是较老但广泛使用的 API

---

## 1. 概述

为给定的聊天对话创建模型响应。Chat Completions API 是 OpenAI 较为成熟的文本生成 API, 但目前 OpenAI 推荐迁移到 Responses API 以获得新功能。

- 消息角色: `system`, `developer`, `user`, `assistant`, `tool`, `function`（已弃用）
- 注意: 参数支持因使用的模型而异, 特别是新的 reasoning 模型有不同要求

---

## 2. 请求体参数

### 2.1 必填参数

#### `messages` — array of ChatCompletionMessageParam

对话消息列表。支持多种消息类型（取决于模型能力）。

#### `model` — string (enum)

模型 ID。支持值示例（非完整列表）:
- `gpt-5.4`, `gpt-5.1`, `gpt-5-pro`
- `gpt-5.1-codex-max`
- `gpt-o4-mini`, `o3`, `o4-mini`
- `gpt-4.1`, `gpt-4o`, `gpt-4o-mini`
- `gpt-image-1`, `gpt-image-1-mini`
- `computer-use-preview`
- `o1-pro`, `o3-pro`

---

### 2.2 可选参数

#### `audio` — ChatCompletionAudioParam

音频输出配置（需要 `modalities: ["audio"]`）。

| 字段 | 类型 | 说明 |
|---|---|---|
| `format` | `"wav"` \| `"aac"` \| `"mp3"` \| `"flac"` \| `"opus"` \| `"pcm16"` | 输出格式 |
| `voice` | `"alloy"` \| `"ash"` \| `"ballad"` \| `"coral"` \| `"echo"` \| `"sage"` \| `"shimmer"` \| `"verse"` \| `"marin"` \| `"cedar"` \| `{id: "voice_1234"}` | 声音 |

#### `frequency_penalty` — number (-2.0 to 2.0)

正数根据已有频率降低重复, 使模型不太可能逐字重复同一段话。

#### `function_call` — DEPRECATED (已弃用)

已弃用, 使用 `tool_choice` 替代。

| 值 | 说明 |
|---|---|
| `"none"` | 不调用函数 |
| `"auto"` | 模型可选择生成消息或调用函数 |
| `{name: "my_function"}` | 强制调用指定函数 |

#### `functions` — DEPRECATED (已弃用)

已弃用, 使用 `tools` 替代。

```json
[{
  "name": "string",
  "description": "string (optional)",
  "parameters": { /* JSON Schema */ }
}]
```

#### `logit_bias` — map of number (-100 to 100)

将 token ID 映射到偏差值。偏差值在采样前加到 logits 上。用于影响特定 token 的生成概率。

#### `logprobs` — boolean

是否返回输出 token 的对数概率。需要时设为 `true`。

#### `max_completion_tokens` — number

生成上限（包括可见输出 token 和推理 token）。推荐替代已弃用的 `max_tokens`。

#### `max_tokens` — DEPRECATED (已弃用)

已弃用, 使用 `max_completion_tokens` 替代。不与 o 系列模型兼容。

#### `metadata` — map of string (最多 16 对键值)

```json
{
  "order_id": "12345",
  "customer_tier": "premium"
}
```

- 键: 最大 64 字符
- 值: 最大 512 字符

#### `modalities` — array of `"text"` | `"audio"` (默认 `["text"]`)

输出模态。支持音频的模型可用 `["text", "audio"]`。

#### `n` — number (1–128)

每个输入消息生成的补全数量。仅当 `n=1` 时流式可用。

#### `parallel_tool_calls` — boolean

是否启用工具调用期间的并行函数调用。

#### `prediction` — ChatCompletionPredictionContent

Predicted Outputs 优化的静态预测输出内容。

```json
{
  "type": "content",
  "content": "Sunset"  // 或 [{ type: "text", text: "Sunset" }, ...]
}
```

#### `presence_penalty` — number (-2.0 to 2.0)

正数根据是否已出现惩罚新 token, 增加讨论新主题的可能性。

#### `prompt_cache_key` — string

OpenAI 用于缓存相似请求的键。替代已弃用的 `user`。

#### `prompt_cache_retention` — `"in_memory"` | `"24h"`

缓存前缀的保留策略。`"24h"` 启用最长 24 小时的扩展缓存。

#### `reasoning_effort` — ReasoningEffort

reasoning 模型的推理努力约束:

| 值 | 说明 |
|---|---|
| `"none"` | 无推理（gpt-5.1 默认） |
| `"minimal"` | 最少推理 |
| `"low"` | 低推理 |
| `"medium"` | 中等推理（gpt-5.1 之前默认） |
| `"high"` | 高推理（gpt-5-pro 默认） |
| `"xhigh"` | 极限推理 |

模型支持差异:
- `gpt-5.1`: 默认 `"none"`, 支持 `none`/`low`/`medium`/`high`
- gpt-5.1 之前: 默认 `"medium"`, 不支持 `"none"`
- `gpt-5-pro`: 仅支持 `"high"`

#### `response_format` — ResponseFormat

- **默认** (`"text"`):
  ```json
  {"type": "text"}
  ```

- **JSON 模式** (`"json_object"`):
  ```json
  {"type": "json_object"}
  ```

- **Structured Outputs** (`"json_schema"`):
  ```json
  {
    "type": "json_schema",
    "json_schema": {
      "name": "calendar",
      "description": "A calendar event",
      "schema": { /* JSON Schema */ },
      "strict": true
    }
  }
  ```

#### `safety_identifier` — string (最大 64 字符)

帮助检测违反使用政策的用户的稳定标识符。建议哈希用户名/邮箱。

#### `seed` — number (BETA, -9223372036854776000 to 9223372036854776000)

确定性采样的最佳努力。不保证完全确定。

#### `service_tier` — `"auto"` | `"default"` | `"flex"` | `"scale"` | `"priority"` (默认 `"auto"`)

处理类型。`"auto"` 使用项目设置。

#### `stop` — string | array of string (最多 4 个)

生成停止的序列。不支持 `o3` 和 `o4-mini`。

#### `store` — boolean

是否存储输出用于模型蒸馏或评估。支持文本和图像输入（超过 8MB 的图像不存储）。

#### `stream` — boolean

是否使用 SSE 流式传输。

#### `stream_options` — ChatCompletionStreamOptions

仅 `stream: true` 时使用:

| 字段 | 类型 | 说明 |
|---|---|---|
| `include_obfuscation` | boolean | 添加随机字符标准化负载大小（旁路攻击缓解） |
| `include_usage` | boolean | 在 `[DONE]` 前流式传输一个额外 chunk 含 token 使用统计 |

#### `temperature` — number (0–2)

采样温度。越高越随机, 越低越集中。推荐改变 `temperature` 或 `top_p` 之一。

#### `tool_choice` — ChatCompletionToolChoiceOption

控制工具调用:

| 值 | 说明 |
|---|---|
| `"none"` | 不调用工具（无工具时默认） |
| `"auto"` | 自动决定（有工具时默认） |
| `"required"` | 必须调用工具 |
| `{type: "function", function: {name: "..."}}` | 强制调用指定函数 |
| `{type: "custom", custom: {name: "..."}}` | 强制调用指定自定义工具 |
| `{type: "allowed_tools", allowed_tools: {mode: "auto"\|"required", tools: [...]}}` | 限制到预定义工具集 |

#### `tools` — array of ChatCompletionTool

```json
[
  {
    "type": "function",
    "function": {
      "name": "get_weather",
      "description": "Get current weather",
      "parameters": { /* JSON Schema */ },
      "strict": true
    }
  },
  {
    "type": "custom",
    "custom": {
      "name": "my_tool",
      "description": "A custom tool",
      "format": {
        "type": "text"
      }
    }
  }
]
```

**自定义工具 format:**
- `TextFormat`: `{type: "text"}`
- `GrammarFormat`: `{type: "grammar", grammar: {definition: "...", syntax: "lark" | "regex"}}`

#### `top_logprobs` — number (0–20)

每个 token 位置返回的最可能 token 数（含对数概率）。需要 `logprobs: true`。

#### `top_p` — number (0–1)

核采样 — 仅考虑累计概率达到 top_p 的 token。

#### `user` — DEPRECATED (已弃用)

被 `safety_identifier` 和 `prompt_cache_key` 替代。

#### `verbosity` — `"low"` | `"medium"` | `"high"`

响应简洁/详细程度控制。越低越简洁。

#### `web_search_options` — object

Web 搜索工具配置:

```json
{
  "search_context_size": "low" | "medium" | "high",
  "user_location": {
    "type": "approximate",
    "city": "San Francisco",
    "country": "US",
    "region": "California",
    "timezone": "America/Los_Angeles"
  }
}
```

- `search_context_size`: 用于搜索的上下文窗口空间指导（默认 `"medium"`）

---

## 3. 消息内容结构

### 3.1 Developer Message

```json
{
  "role": "developer",
  "content": "You are a helpful assistant.",
  "name": "optional_name"
}
```

- content: `string` 或 `[{type: "text", text: "..."}]` (仅 text 类型)
- 用于 o1/o3/o4 系列模型, 取代 `system` 角色

### 3.2 System Message (Legacy)

```json
{
  "role": "system",
  "content": "You are a helpful assistant.",
  "name": "optional_name"
}
```

- 推荐对较新模型使用 `developer` 角色替代

### 3.3 User Message

```json
{
  "role": "user",
  "content": "string"  // 或 content parts 数组
}
```

**Content parts (content 为数组时的元素类型):**

**文本:**
```json
{"type": "text", "text": "What's in this image?"}
```

**图片 (URL):**
```json
{
  "type": "image_url",
  "image_url": {
    "url": "https://example.com/image.jpg",
    "detail": "auto" | "low" | "high"
  }
}
```

**图片 (Base64):**
```json
{
  "type": "image_url",
  "image_url": {
    "url": "data:image/jpeg;base64,<data>"
  }
}
```

**音频输入:**
```json
{
  "type": "input_audio",
  "input_audio": {
    "data": "<base64>",
    "format": "wav" | "mp3"
  }
}
```

**文件输入:**
```json
{
  "type": "file",
  "file": {
    "file_data": "<base64>",  // 或 file_id
    "filename": "report.pdf"
  }
}
```

### 3.4 Assistant Message

```json
{
  "role": "assistant",
  "content": "string" | [text parts | refusal parts],
  "audio": {"id": "a_1234"},
  "name": "optional",
  "tool_calls": [
    {
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\":\"SF\"}"
      }
    }
  ],
  "refusal": null
}
```

### 3.5 Tool Message

```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "72°F, sunny"
}
```

### 3.6 Function Message (Legacy, 已弃用)

```json
{
  "role": "function",
  "name": "get_weather",
  "content": "72°F, sunny"
}
```

---

## 4. 响应示例

非流式:
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4o",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you today?",
      "tool_calls": null,
      "function_call": null
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 9,
    "completion_tokens": 12,
    "total_tokens": 21
  }
}
```

`finish_reason` 可能值: `"stop"`, `"length"`, `"tool_calls"`, `"content_filter"`, `"function_call"`

---

## 5. 全部请求参数速查

| # | 参数 | 类型 | 必填 | 状态 | 说明 |
|---|---|---|---|---|---|
| 1 | `model` | string | **是** | Active | 模型 ID |
| 2 | `messages` | array | **是** | Active | 对话消息 |
| 3 | `audio` | object | 否 | Active | 音频输出配置 |
| 4 | `frequency_penalty` | number | 否 | Active | 频率惩罚 |
| 5 | `logit_bias` | map | 否 | Active | Token 偏差 |
| 6 | `logprobs` | boolean | 否 | Active | 返回 logprobs |
| 7 | `max_completion_tokens` | number | 否 | Active | 最大输出 token |
| 8 | `metadata` | map (≤16) | 否 | Active | 元数据 |
| 9 | `modalities` | array | 否 | Active | 输出模态 |
| 10 | `n` | number (1-128) | 否 | Active | 候选数 |
| 11 | `parallel_tool_calls` | boolean | 否 | Active | 并行工具调用 |
| 12 | `prediction` | object | 否 | Active | Predicted Outputs |
| 13 | `presence_penalty` | number | 否 | Active | 存在惩罚 |
| 14 | `prompt_cache_key` | string | 否 | Active | 缓存键 |
| 15 | `prompt_cache_retention` | string | 否 | Active | 缓存保留 |
| 16 | `reasoning_effort` | string | 否 | Active | 推理努力 |
| 17 | `response_format` | object | 否 | Active | 输出格式 |
| 18 | `safety_identifier` | string | 否 | Active | 用户标识 |
| 19 | `seed` | number | 否 | Beta | 确定性种子 |
| 20 | `service_tier` | string | 否 | Active | 服务等级 |
| 21 | `stop` | string\|array | 否 | Active | 停止序列 |
| 22 | `store` | boolean | 否 | Active | 存储响应 |
| 23 | `stream` | boolean | 否 | Active | 流式 |
| 24 | `stream_options` | object | 否 | Active | 流式选项 |
| 25 | `temperature` | number (0-2) | 否 | Active | 温度 |
| 26 | `tool_choice` | string\|object | 否 | Active | 工具选择 |
| 27 | `tools` | array | 否 | Active | 工具定义 |
| 28 | `top_logprobs` | number (0-20) | 否 | Active | Top logprobs |
| 29 | `top_p` | number (0-1) | 否 | Active | 核采样 |
| 30 | `verbosity` | string | 否 | Active | 详细程度 |
| 31 | `web_search_options` | object | 否 | Active | Web 搜索配置 |
| 32 | `function_call` | string\|object | 否 | **Deprecated** | → `tool_choice` |
| 33 | `functions` | array | 否 | **Deprecated** | → `tools` |
| 34 | `max_tokens` | number | 否 | **Deprecated** | → `max_completion_tokens` |
| 35 | `user` | string | 否 | **Deprecated** | → `safety_identifier` |

# Anthropic Messages vs OpenAI Chat Completions vs OpenAI Responses — 协议对比报告

> 基于 2026年5月 官方文档生成

---

## 1. 概览

| 维度 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses |
|---|---|---|---|
| **端点** | `POST /v1/messages` | `POST /v1/chat/completions` | `POST /v1/responses` |
| **定位** | 消息级 API | 聊天补全 API（成熟, 被 Responses 逐步替代） | 新一代统一 API（OpenAI 推荐） |
| **会话模型** | 无状态, messages 数组手动管理 | 无状态, messages 数组手动管理 | 有状态, conversation + 多态 input items |
| **角色** | user, assistant（无 system role, 用 top-level system 参数） | system, developer, user, assistant, tool, function | user, assistant, system, developer（InputItem 多态） |

---

## 2. 请求参数映射表

### 2.1 核心参数

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **模型** | `model` (string/enum) | `model` (string/enum) | `model` (string/enum) | 直接映射 |
| **输入** | `messages` (array of `{role, content}`) | `messages` (array of message objects) | `input` (string | array of InputItem) | 格式差异大，见 §3 详述 |
| **系统提示** | `system` (string | array of text blocks) | `messages[].role="system"/"developer"` | `instructions` (string) | 结构不同，见 §3.7 |
| **最大输出** | `max_tokens` (number, 必填) | `max_completion_tokens` (number, 可选) | `max_output_tokens` (integer, 可选, min=16) | 可直接映射 |
| **温度** | `temperature` (0.0–1.0, 默认 1.0) | `temperature` (0–2, 默认 1) | `temperature` (0–2, 默认 1) | 值范围不同，Anthropic 上限 1.0 |
| **Top-P** | `top_p` (number) | `top_p` (number, 0–1) | `top_p` (number, 0–1) | 直接映射 |
| **Top-K** | `top_k` (number) | ❌ 不支持 | ❌ 不支持 | Anthropic 专有 |
| **停止序列** | `stop_sequences` (array of string) | `stop` (string | array, 最多 4 个) | ❌ 不支持 | Chat → Anthropic 可映射; Responses 不支持 |

### 2.2 推理 / 思考

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **推理/思考** | `thinking` (`{type, budget_tokens, display}`) | `reasoning_effort` (枚举) | `reasoning` (`{effort, summary}`) | 概念相似，参数结构不同 |
| **Thinking 模式** | `enabled` / `disabled` / `adaptive` | — | — | Anthropic 专有 |
| **推理努力** | `budget_tokens` (token 数量预算) | `reasoning_effort` (none/minimal/low/medium/high/xhigh) | `reasoning.effort` (同 Chat) | 概念不同: 预算 vs 努力级别; 不可直接相互转换 |
| **推理摘要** | `thinking.display` (summarized/omitted) | ❌ | `reasoning.summary` (auto/concise/detailed) | 功能不同 |
| **Thinking Block** | `ThinkingBlockParam` / `RedactedThinkingBlockParam` (可在 messages 中传递) | — | `reasoning` InputItem | 需要转换 |

### 2.3 工具调用

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **工具定义** | `tools` (array of ToolUnion) | `tools` (array of `{type, function}` or `{type, custom}`) | `tools` (array of Tool, 多态) | 见 §4 详述 |
| **工具选择** | `tool_choice` (`{type: auto/any/tool/none}`) | `tool_choice` (none/auto/required 或按名称) | `tool_choice` (none/auto/required 或按名称/类型) | 概念相同，细节不同 |
| **并行调用** | `tool_choice.disable_parallel_tool_use` (默认 false) | `parallel_tool_calls` (boolean) | `parallel_tool_calls` (boolean, 默认 true) | 可直接映射 |
| **工具结果** | `ToolResultBlockParam` (含 is_error) | `tool` role message (content string, tool_call_id) | `function_call_output` InputItem (含 output, status) | 格式差异大 |
| **工具引用** | `ToolReferenceBlockParam` | ❌ | — | Anthropic 专有 |
| **工具搜索** | ToolSearchTool | ❌ | ToolSearchTool | Chat 不支持 |
| **内置服务端工具** | web_search, web_fetch, code_execution, bash, text_editor, memory | 无内置 (仅 web_search_options 配置) | web_search, file_search, code_interpreter, image_generation, computer, mcp, local_shell, apply_patch 等 | Anthropic 有丰富内置; Chat 最少; Responses 最丰富 |

### 2.4 输出格式 / 结构化输出

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **JSON 模式** | ❌ (无独立的 JSON mode) | `response_format: {type: "json_object"}` | `text.format: {type: "json_object"}` | Anthropic 不支持传统 JSON 模式 |
| **JSON Schema** | `output_config.format: {type: "json_schema", schema}` | `response_format: {type: "json_schema", json_schema: {name, schema, strict}}` | `text.format: {type: "json_schema", name, schema, strict}` | 结构兼容，可直接映射 |
| **输出努力** | `output_config.effort` (low/medium/high/xhigh/max) | ❌ | ❌ | Anthropic 专有 |
| **语法格式** | ❌ | `custom.format: {type: "grammar", grammar}` | `text.format: {type: "grammar", grammar}` | Anthropic 不支持 |
| **Verbosity** | ❌ | `verbosity` (low/medium/high) | `text.verbosity` (low/medium/high) | Anthropic 不支持 |

### 2.5 采样高级参数

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **频率惩罚** | ❌ | `frequency_penalty` (-2.0–2.0) | ❌ | 仅 Chat 支持 |
| **存在惩罚** | ❌ | `presence_penalty` (-2.0–2.0) | ❌ | 仅 Chat 支持 |
| **Logit Bias** | ❌ | `logit_bias` (map) | ❌ | 仅 Chat 支持 |
| **Logprobs** | ❌ | `logprobs` + `top_logprobs` | `top_logprobs` | Anthropic 不支持 |
| **Seed** | ❌ | `seed` (number) | ❌ | 仅 Chat 支持 |
| **N (多候选)** | ❌ | `n` (1–128) | ❌ | 仅 Chat 支持 |
| **预填充输出** | ❌ | `prediction: {content, type: "content"}` | ❌ | 仅 Chat 支持（Predicted Outputs） |

### 2.6 上下文 / 缓存 / 会话

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **Prompt 缓存** | `cache_control` (ephemeral, TTL: 5m/1h, block-level) | `prompt_cache_key` + `prompt_cache_retention` (request-level) | `prompt_cache_key` + `prompt_cache_retention` | 机制完全不同: block-level vs request-level |
| **容器复用** | `container` (string) | ❌ | — | Anthropic 专有 |
| **推理地理** | `inference_geo` (string) | ❌ | ❌ | Anthropic 专有 |
| **服务等级** | `service_tier` (auto/standard_only) | `service_tier` (auto/default/flex/scale/priority) | `service_tier` (同 Chat) | 选项不同 |
| **截断** | ❌ | ❌ | `truncation` (auto/disabled) | Responses 专有 |
| **会话管理** | 无状态 (手动管理 messages) | 无状态 (手动管理 messages) | `conversation` (会话 ID) + `previous_response_id` | Responses 有状态, 其他无状态 |
| **上下文压缩** | ❌ | ❌ | `context_management` (compaction) | Responses 专有 |
| **后台模式** | ❌ | ❌ | `background` (boolean) | Responses 专有 |
| **存储** | ❌ | `store` (boolean) | `store` (boolean, 默认 true) | Anthropic 不支持 |
| **Prompt 模板** | ❌ | ❌ | `prompt` ({id, version, variables}) | Responses 专有 |

### 2.7 流式传输

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **流式开关** | `stream` (boolean) | `stream` (boolean) | `stream` (boolean) | 直接映射 |
| **流式选项** | — | `stream_options: {include_usage, include_obfuscation}` | `stream_options: {include_obfuscation}` | Chat 最丰富 |
| **事件格式** | Anthropic SSE 事件格式 | OpenAI SSE 事件格式 (`data: [DONE]`) | OpenAI SSE 事件格式 | 事件结构不同，需要翻译 |

### 2.8 用户/元数据

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **用户标识** | `metadata.user_id` | `safety_identifier` (推荐) / `user` (已弃用) | `safety_identifier` / `user` (已弃用) | 位置和命名不同 |
| **元数据** | 仅 `metadata.user_id` | `metadata` (map, 最多 16 对键值) | `metadata` (map, 最多 16 对键值) | Anthropic 仅支持 user_id |

### 2.9 音频

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 映射关系 |
|---|---|---|---|---|
| **输入音频** | ❌ | `input_audio` content part | — | 仅 Chat 支持音频输入 |
| **输出音频** | ❌ | `modalities: ["audio"]` + `audio: {format, voice}` | — | 仅 Chat 支持音频输出 |

### 2.10 多模态输入

| 功能领域 | Anthropic Messages | OpenAI Chat Completions | OpenAI Responses |
|---|---|---|---|
| **图片 (URL)** | `ImageBlockParam` (URLImageSource) | `image_url` content part (`{url, detail}`) | `input_image` content (`{image_url, file_id, detail}`) |
| **图片 (Base64)** | `ImageBlockParam` (Base64ImageSource) | `image_url` content part (data: URL) | `input_image` content (data: URL) |
| **文件** | `DocumentBlockParam` (PDF/Text, Base64/URL) | `file` content part (`{file_id, file_data}`) | `input_file` content (`{file_id, file_data, file_url}`) |
| **容器上传** | `ContainerUploadBlockParam` | ❌ | — |

---

## 3. 消息/输入格式映射详述

### 3.1 角色映射

| Anthropic Messages | OpenAI Chat Completions | OpenAI Responses | 说明 |
|---|---|---|---|
| `user` | `user` | `user` (message input) | 直接映射 |
| `assistant` | `assistant` | `assistant` (output_message) | 直接映射 |
| — (top-level `system`) | `system` / `developer` | `system` / `developer` (message input) | Anthropic 没有 system role, 用参数 |
| — | `tool` | `function_call_output` | Anthropic 有 `tool_result` |
| — | `function` | — | Chat 旧版, 已弃用 |

### 3.2 文本内容

| Anthropic | OpenAI Chat | OpenAI Responses |
|---|---|---|
| `{type: "text", text: "..."}` | `{type: "text", text: "..."}` | `{type: "input_text", text: "..."}` |
| 支持 citations | 不支持 citations | — |

### 3.3 图片内容

```
Anthropic → OpenAI:
  ImageBlockParam(source=Base64ImageSource) 
    → content: [{type: "image_url", image_url: {url: "data:{media_type};base64,{data}"}}]
  ImageBlockParam(source=URLImageSource)
    → content: [{type: "image_url", image_url: {url: "..."}}]

注意: Anthropic 明确支持 JPEG/PNG/GIF/WebP; OpenAI 也支持这些但 detail 参数 Anthropic 没有
```

### 3.4 文档/文件内容

```
Anthropic DocumentBlock → OpenAI Responses input_file / Chat file content part
  Base64PDFSource    → file_data (base64 pdf)
  PlainTextSource    → text content
  URLPDFSource       → file_url
  ContentBlockSource → 不支持直接转换（嵌套的 content blocks 无 OpenAI 等价物）
```

### 3.5 工具调用流转

```
Anthropic 流程:
  assistant: [tool_use( id, name, input )]   ← 模型请求
  user:     [tool_result( tool_use_id, content, is_error )]  ← 用户返回

OpenAI Chat 流程:
  assistant: { tool_calls: [{ id, function: { name, arguments }, type: "function" }] }
  tool:      { role: "tool", tool_call_id: id, content: "..." }

OpenAI Responses 流程:
  function_call item:        { type: "function_call", call_id, name, arguments }
  function_call_output item: { type: "function_call_output", call_id, output }

关键差异:
  - Anthropic 使用 content blocks (tool_use / tool_result) 在 messages 内部
  - Chat 使用独立的 role="tool" message
  - Responses 使用独立的 item types
```

### 3.6 工具定义映射

```
Anthropic Tool → OpenAI FunctionTool:
  name         → name
  description  → description
  input_schema → parameters (JSON Schema)
  strict       → strict
  defer_loading → (Responses 支持, Chat 不支持)

Anthropic 额外支持:
  - type: "custom" (可选标记, OpenAI 无等价)
  - allowed_callers (限定调用者)
  - eager_input_streaming (流式工具输入)
  - input_examples (工具用例)

OpenAI 额外支持:
  - custom tool (非 function 类型, 用 grammar/text 格式)
```

### 3.7 系统提示映射

```
Anthropic → OpenAI Chat:
  system: "You are helpful"  →  messages: [{role: "system", content: "You are helpful"}]
  system: [{type: "text", text: "part1"}, {type: "text", text: "part2"}]  
    → messages: [{role: "system", content: "part1\npart2"}]

Anthropic → OpenAI Responses:
  system: "..."  →  instructions: "..."

差异:
  - Anthropic 支持 system 为 text block 数组（含 cache_control）
  - Chat 支持 system 和 developer 两种角色
  - Responses 使用 instructions 独立参数
  - Anthropic 暂不支持 developer 角色概念
```

---

## 4. 无法转换的功能清单

### 4.1 Anthropic → OpenAI 无法实现

| Anthropic 功能 | 原因 | 影响 |
|---|---|---|
| **`top_k` 采样** | OpenAI 不支持 Top-K 采样 | 采样行为不可完全复现 |
| **Block-level Prompt 缓存** (`cache_control` on content blocks) | OpenAI 仅支持 request-level 缓存 key | 缓存粒度不同, 不可精准对应 |
| **Extended Thinking** (`thinking: {type: "enabled", budget_tokens}`) | 概念不同: token 预算 vs 努力级别 | 不可精确映射, 需近似 |
| **Adaptive Thinking** (`thinking: {type: "adaptive"}`) | OpenAI 无"自适应"概念 | 无法映射 |
| **Thinking/RedactedThinking blocks 在 messages 中传递** | 消息结构不兼容 | 多轮对话中的思考上下文丢失 |
| **Server-side tools** (web_search, web_fetch, code_execution, bash, memory) | 内置服务端工具需 Anthropic 侧执行 | Chat 无等价内置工具; Responses 有类似但不同 |
| **Container reuse** (`container`) | OpenAI 无容器概念 | 无法复用计算上下文 |
| **`inference_geo`** | OpenAI 无按请求指定推理区域 | 无法控制数据驻留 |
| **Document citations** (`citations` on text/docs) | OpenAI 无内置 citation 系统 | citation 数据丢失 |
| **ToolReferenceBlockParam** | OpenAI 无工具引用概念 | 无法传递工具参考 |
| **`output_config.effort`** | OpenAI 无输出努力参数 | 无法精确控制 |
| **`defer_loading`** on tools | Chat 不支持; Responses 支持 | Chat 模式下大工具集问题 |

### 4.2 OpenAI Chat → Anthropic 无法实现

| Chat 功能 | 原因 |
|---|---|
| **`frequency_penalty`** / **`presence_penalty`** | Anthropic 没有这些采样参数 |
| **`logit_bias`** (token 级 bias) | Anthropic 不支持 |
| **`logprobs`** / **`top_logprobs`** | Anthropic 不支持返回 logprobs |
| **`seed`** (确定性) | Anthropic 不支持 seed 参数 |
| **`n`** (多候选) | Anthropic 不支持一次请求返回多个候选 |
| **`modalities: ["audio"]`** / `audio` (语音输出) | Anthropic 不支持语音输出 |
| **`input_audio`** (语音输入) | Anthropic 不支持音频输入 |
| **`service_tier`** 多级别 (flex/scale/priority) | Anthropic 仅 auto/standard_only |
| **`developer` role** | Anthropic 无 developer 角色, 需映射到 system 参数 |
| **`verbosity`** | Anthropic 无简洁/详细控制参数 |
| **Custom tools** (grammar format) | Anthropic 仅支持 JSON Schema function 工具 |
| **Predicted Outputs** | Anthropic 无等价的预填充输出优化 |
| **`prompt_cache_retention: "24h"`** | Anthropic cache TTL 最长为 1h |
| **`store`** (存储响应) | Anthropic 不存储响应用于后续检索 |

### 4.3 OpenAI Responses → Anthropic 无法实现

| Responses 功能 | 原因 |
|---|---|
| **`conversation`** (有状态会话) + `previous_response_id` | Anthropic 完全无状态 |
| **`truncation`** (自动截断) | Anthropic 无自动截断 |
| **`context_management`** (compaction) | Anthropic 无自动上下文压缩 |
| **`background`** (后台执行) | Anthropic 不支持后台模式 |
| **`store`** (响应持久化 + 检索) | Anthropic 无响应存储 |
| **`prompt`** (模板) | Anthropic 无 prompt 模板系统 |
| **多态 InputItem** 类型 (file_search_call, computer_call, image_generation_call, etc.) | Anthropic 的消息结构不支持这些 item 类型作为独立一级单元 |
| **Built-in File Search tool** | Anthropic 无内置文件搜索工具 |
| **Computer Use tool** (带 display/environment) | Anthropic 无内置 computer use 工具 (需作为 custom tool) |
| **Image Generation tool** | Anthropic 无内置图像生成工具 |
| **MCP tool** (connector/SaaS 集成) | Anthropic 无内置 MCP 连接器 |
| **Apply Patch tool** | Anthropic 无内置 |
| **Local Shell tool** | Anthropic 无内置; 需使用 bash_20250124 |
| **`include`** (请求额外输出数据) | Anthropic 无等价的 include 机制 |
| **`max_tool_calls`** | Anthropic 通过 disable_parallel_tool_use 间接控制, 无精确调用上限 |

### 4.4 三者共有但参数不同的功能

| 功能 | Anthropic | OpenAI Chat | OpenAI Responses | 差异 |
|---|---|---|---|---|
| Temperature 范围 | 0.0 – 1.0 | 0 – 2 | 0 – 2 | Anthropic 最大值仅 1.0 |
| max_tokens 参数名 | `max_tokens` (必填) | `max_completion_tokens` (可选) | `max_output_tokens` (可选) | 名称和必填性不同 |
| Stream 事件格式 | Anthropic SSE (自有格式) | OpenAI SSE (`data: [DONE]`) | OpenAI SSE (同 Chat) | 事件结构不兼容 |
| 工具 result 结构 | content blocks 内 `tool_result` | 独立 `role="tool"` message | 独立 `function_call_output` item | 结构完全不同 |
| 缓存 | block-level ephemeral | request-level key + retention | request-level key + retention | 粒度完全不同 |

---

## 5. 转换建议

### 5.1 Anthropic → OpenAI (Chat Completions)

```
推荐路径: Anthropic Messages → OpenAI Chat Completions
转换复杂度: 中等

✅ 可转换:
  - model: 直接映射到目标模型
  - messages: 角色映射 (user→user, assistant→assistant)
  - system: 插入为 role="system" message
  - max_tokens → max_completion_tokens
  - temperature: 注意范围调整 (Anthropic 0-1 vs OpenAI 0-2)
  - top_p, stop_sequences: 直接映射
  - tools: 映射为 {type: "function", function: {name, description, parameters}}
  - tool_choice: auto/any→auto, tool→function name, none→none
  - stream: 直接映射 (但需翻译事件)
  - thinking: 近似映射到 reasoning_effort
  - structured output: output_config.format → response_format

⚠️ 有损转换:
  - cache_control (block-level) → prompt_cache_key (request-level): 丢失精确位置
  - top_k: 丢失
  - thinking.budget_tokens → reasoning_effort: 不可精确, 需启发式映射
  - server_tools (web_search 等) → web_search_options: 功能子集
  - citations: 完全丢失
```

### 5.2 Anthropic → OpenAI (Responses)

```
推荐路径: Anthropic Messages → OpenAI Responses
转换复杂度: 较高（消息/项目结构差异大）

除了 Chat 的转换规则外:
  ⚠️ 额外挑战:
  - messages 数组 → input items 数组: 结构完全不同,需双向解析
  - system → instructions: 更自然
  - 无 conversation 状态管理: 需自行维护
  - output_config.format → text.format: 参数路径不同

✅ 比 Chat 更好的地方:
  - defer_loading: Responses 支持
  - 更多内置工具选择
  - store/background 等高级特性 Anthropic 侧无影响
```

### 5.3 OpenAI (Chat/Responses) → Anthropic

```
推荐路径: OpenAI Chat Completions → Anthropic Messages
转换复杂度: 中等

❌ 完全丢失:
  - frequency_penalty, presence_penalty
  - logit_bias, logprobs
  - seed, n
  - modalities (audio)
  - input_audio
  - Predicted Outputs
  - verbosity
  - store
  - developer role → system parameter (近似)

⚠️ 有损转换:
  - prompt_cache_retention (24h) → cache_control ttl (最大 1h)
  - service_tier flex/scale → standard_only (近似)
  - custom tools (grammar) → 无法转换
```

### 5.4 Responses → Chat Completions

```
推荐路径: (不推荐, 但可降级)
转换复杂度: 高

主要原因:
  - Responses 的多态 InputItem 在 Chat 中没有等价物
  - conversation / previous_response_id 无状态管理能力在 Chat 需手动实现
  - 大量内置工具 (MCP, File Search, Computer Use, Apply Patch 等) Chat 不支持
  - truncation, compaction, background 等特性 Chat 全不支持
```

---

## 6. 决策矩阵

| 需求场景 | 推荐方案 |
|---|---|
| 需要 Anthropic Claude 模型 | 使用 Anthropic Messages API 原生 |
| 需要 OpenAI GPT/o-series 模型 | 使用 OpenAI Responses API (新一代) |
| 需要音频输入/输出 | 仅 OpenAI Chat Completions |
| 需要确定性采样 (seed) | 仅 OpenAI Chat Completions |
| 需要多候选 (n>1) | 仅 OpenAI Chat Completions |
| 需要 token 级 logprobs | 仅 OpenAI (Chat/Responses) |
| 需要有状态会话管理 | 仅 OpenAI Responses |
| 需要 MCP 连接器/SaaS 集成 | 仅 OpenAI Responses |
| 需要 Computer Use / Image Generation | 仅 OpenAI Responses |
| 需要精细缓存控制 (block-level) | 仅 Anthropic Messages |
| 需要 server tools (内置 web_search 等) | Anthropic 或 OpenAI Responses |
| 需要 Anthropic 思考/引用透明度 | 仅 Anthropic Messages |
| 作为统一网关转换 | Chat Completions 为中间层最成熟 |

---

*报告生成时间: 2026年5月*

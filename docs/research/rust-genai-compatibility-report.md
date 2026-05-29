# rust-genai 三种 API 兼容性调研报告

> 库: `https://github.com/jeremychone/rust-genai` (v0.6.0, 2026-05-23)
> 调研范围: `AdapterKind::Anthropic` / `AdapterKind::OpenAI` / `AdapterKind::OpenAIResp`

---

## 1. 总体评价

**rust-genai 对三种协议的核心差异抹平做得相当好**，对常用场景（聊天、工具调用、流式、JSON Schema）覆盖完整。但它**不追求 100% 覆盖每个 provider 的功能**——这正是其设计定位（README 明确说明 "focuses on standardizing chat completion APIs across major AI providers while preserving provider-specific strengths"）。

**在三种 API 中，Anthropic 和 OpenAI Chat 兼容性最好，OpenAI Responses 由于 API 本身更丰富，有不少额外特性未能完整映射。**

---

## 2. 请求参数映射覆盖度

| 参数 | Anthropic | OpenAI Chat | OpenAI Resp | 说明 |
|---|---|---|---|---|
| `temperature` | ✅ 直接映射 | ✅ 直接映射 | ✅ 直接映射 | |
| `max_tokens` | ✅ `"max_tokens"` | ✅ `"max_completion_tokens"` (新模型) | ✅ `"max_output_tokens"` | 各路名称不同，均已处理 |
| `top_p` | ✅ `"top_p"` | ✅ `"top_p"` | ✅ `"top_p"` | |
| `stop_sequences` | ✅ `"stop_sequences"` | ✅ `"stop"` | ❌ 不支持 | Responses API 不支持 stop，非 genai 局限 |
| `reasoning_effort` | ✅ 三级映射（effort API / adaptive / legacy thinking） | ✅ 直接映射 | ✅ `reasoning.effort` | Anthropic 映射最复杂但最完整 |
| `response_format` (JsonMode) | ❌ 静默忽略 | ✅ `json_object` | ✅ `text.format: json_object` | Anthropic 无传统 JSON 模式 |
| `response_format` (JsonSpec) | ✅ `output_config.format` | ✅ `json_schema` | ✅ `text.format: json_schema` | 三人均有但字段结构不同 |
| `tool_choice` | ✅ Auto/Any/None/Tool | ✅ Auto/Required/None/Function | ✅ Auto/Required/None/Function | 全部覆盖 |
| `seed` | ❌ | ✅ `"seed"` | ✅ `"seed"` | Anthropic 原生无 seed |
| `service_tier` | ❌ | ✅ `"service_tier"` | ❌ | 仅 OpenAI Chat 支持 |
| `verbosity` | ❌ | ✅ `"verbosity"` | ✅ `text.verbosity` | Anthropic 无 verbosity 概念 |
| `top_k` | ❌ | ❌ | ❌ | 三个 adapter 都未实现 |
| `frequency_penalty` | ❌ | ❌ | ❌ | 三个 adapter 都未实现 |
| `presence_penalty` | ❌ | ❌ | ❌ | 三个 adapter 都未实现 |
| `logit_bias` | ❌ | ❌ | ❌ | 三个 adapter 都未实现 |
| `cache_control` (message-level) | ✅ block-level 带 TTL | ❌ | ❌ | Anthropic 独有支持 |
| `cache_control` (request-level) | ❌ 被忽略 | ✅ `prompt_cache_retention` | ✅ `prompt_cache_retention` | |
| `prompt_cache_key` | ❌ | ✅ | ✅ | Anthropic 无此概念 |
| `extra_body` | ❌ | ✅ 低层逃逸 | ✅ 低层逃逸 | Anthropic 不支持 |

### 总结：genai ChatOptions 只覆盖了三者**共有**的子集，provider-specific 参数不在抽象层

---

## 3. 消息 / 内容块映射

### 3.1 系统提示

| Adapter | 映射方式 |
|---|---|
| **Anthropic** | 顶层 `"system"` 属性（多段用 `\n\n` 拼接；有 cache 时变成 text block 数组） |
| **OpenAI Chat** | 插入为 `messages[0]` 的 `role: "system"` |
| **OpenAI Resp** | **同时**设置 `"instructions"` + inline system items |

### 3.2 用户消息

| 内容 | Anthropic | OpenAI Chat | OpenAI Resp |
|---|---|---|---|
| 纯文本 | `{type: "text"}` | `{type: "text"}` | `{type: "input_text"}` |
| 图片 (base64) | `{type: "image", source: {type: "base64", ...}}` | `{type: "image_url", image_url: {url: "data:..."}}` | `{type: "input_image", image_url: "data:..."}` |
| 图片 (URL) | ⚠️ **warn! 跳过** | ✅ `image_url: {url: "..."}` | ✅ `input_image, image_url: "..."` |
| 音频 | ❌ 静默跳过 | ✅ `{type: "input_audio"}` | 🚧 TODO 未处理 |
| 文件/文档 | `{type: "document", source: {type: "base64"/"url", ...}}` | `{type: "file", file: {filename, file_data}}` | `{type: "input_file", filename, file_data/file_url}` |
| tool_use ↔ tool_result | `{role: "user", content: [{type: "tool_result", ...}]}` | `{role: "tool", content, tool_call_id}` | 扁平化 `{type: "function_call_output", call_id, output}` |

### 3.3 Assistant 消息

| 内容 | Anthropic | OpenAI Chat | OpenAI Resp |
|---|---|---|---|
| 文本 | `{type: "text"}` | 直接 string（多段用 `\n\n` 拼接） | `{type: "output_text"}` 在 `message` item 内 |
| tool_use | `{type: "tool_use", id, name, input}` | `{id, type: "function", function: {name, arguments}}` | **扁平化**为独立 `{type: "function_call"}` item |
| Thinking/Reasoning | `{type: "thinking"}` → 合并入 `reasoning_content` | `reasoning_content` → 合并入消息级 `reasoning_content` 字段 | 已签名 → 预处理为 `{type: "reasoning"}` item（round-trip） |
| 多个工具调用 | 内嵌在 assistant 内容中 | 内嵌在 assistant 内容中 | **混合内容拆分为消息 + 各自独立 function_call items** |

### 3.4 关键差异

- **OpenAI Resp 的消息转换最复杂**：tool_call 和文本不在同一个 item 中
- **Anthropic 图片 URL 不支持**（只支持 base64）
- **audio 仅 OpenAI Chat 支持**
- **三种 adapter 对 `ThoughtSignature`、`ReasoningContent`、`Custom` 在消息构建时均静默跳过**

---

## 4. 工具映射

### 4.1 工具定义

| 方面 | Anthropic | OpenAI Chat | OpenAI Resp |
|---|---|---|---|
| 自定义工具 Schema 字段 | `"input_schema"` | `"function.parameters"`（嵌套） | `"parameters"`（扁平） |
| strict 模式 | 不设置 | 遍历 schema 加 `additionalProperties: false` | 同 Chat |
| Web Search 内置 | ✅ `web_search_20250305` 含 domain 过滤、max_uses | ❌ 无内置 | ✅ `"web_search"` 但 domain 过滤未完成 |

### 4.2 工具选择

| GenAI | Anthropic | OpenAI Chat | OpenAI Resp |
|---|---|---|---|
| `Auto` | `{type: "auto"}` | `"auto"` | `"auto"` |
| `None` | `{type: "none"}` | `"none"` | `"none"` |
| `Required` | `{type: "any"}` | `"required"` | `"required"` |
| `Tool{name}` | `{type: "tool", name}` | `{type: "function", function: {name}}` | `{type: "function", name}` |

---

## 5. 流式事件对比

| 维度 | Anthropic | OpenAI Chat | OpenAI Resp |
|---|---|---|---|
| **传输格式** | Anthropic SSE | OpenAI SSE (`data: [DONE]`) | OpenAI SSE |
| **结束标记** | `message_start` → … → `message_stop` | 逐 delta chunk，`[DONE]` 结束 | 逐类型 event，`response.completed` 结束 |
| **文本 chunk** | `content_block_delta` on text block | `choices[0].delta.content` | `response.output_text.delta` |
| **推理/思考 chunk** | `thinking` 转为 `ReasoningChunk` | `choices[0].delta.reasoning_content` | `response.reasoning_text.delta` + `response.reasoning_summary_text.delta` |
| **工具调用** | 增量：`content_block_start`(name+id) → `content_block_delta`(args) → `content_block_stop`(finalize) | 累积：`choices[0].delta.tool_calls[]` 按 idx 累积 → final parse | 增量：`response.output_item.added`(track) → `response.function_call_arguments.delta`(增量) → `response.output_item.done`(finalize) |
| **停止原因** | `message_delta.stop_reason` | `choices[0].finish_reason` | `response.completed` with status |
| **Usage** | `message_start` + `message_delta` | 随 `finish_reason` 或最终 standalone usage | `response.completed` |
| **动签(ThoughtSignature)** | 流内不捕获 | 不支持 | ✅ 流内从 `reasoning` output_items 捕获 |

---

## 6. 未能实现的功能（按 API 维度）

### 6.1 Anthropic 独有 → genai 未暴露

| 功能 | 状态 |
|---|---|
| `top_k` | ❌ |
| block-level `cache_control` 精确 TTL 控制 (24h → 1h 降级) | ⚠️ 降级 |
| 消息内 citations | ❌ |
| `container` (容器复用) | ❌ |
| `inference_geo` | ❌ |
| server-side tools: bash, text_editor, memory, web_fetch 等 | partially ✅ web_search |
| `tool_choice.disable_parallel_tool_use` | ❌ |
| `output_config.effort` 完整 range (Max/Budget 部分 ok) | ⚠️ 部分 |
| `thinking` adaptive 模式的 display 控制 | ❌ |

### 6.2 OpenAI Chat 独有 → genai 未暴露

| 功能 | 状态 |
|---|---|
| `frequency_penalty` / `presence_penalty` | ❌ |
| `logit_bias` | ❌ |
| `n`（多候选） | ❌ |
| `logprobs` / `top_logprobs` | ❌ |
| `modalities: ["audio"]` | ❌（音频输出） |
| `input_audio` | ✅ 作为 content part 使用 |
| `prediction`（Predicted Outputs） | ❌ |
| `store` | ❌ |
| `developer` role 区分于 `system` | ❌ |
| `custom tools` (grammar format) | ❌ |
| `stream_options.include_usage` / `include_obfuscation` | ❌ |

### 6.3 OpenAI Responses 独有 → genai 未暴露

| 功能 | 状态 |
|---|---|
| `conversation`（有状态会话 ID） | ❌ 但有 `previous_response_id` + `store` |
| `truncation`（自动截断） | ❌ |
| `context_management`（compaction） | ❌ |
| `background`（后台执行） | ❌ |
| `prompt`（模板） | ❌ |
| `include`（额外输出数据请求） | ❌ |
| `max_tool_calls` | ❌ |
| 内置工具：file_search, code_interpreter, image_generation, computer, MCP, local_shell, apply_patch | ❌ |
| Web Search domain 过滤 (`ToolConfig::WebSearch`) | 🚧 FIXME 未实现 |
| Audio 二进制输入 | 🚧 TODO 未实现 |
| ReasoningSummary text delta 单独处理 | ⚠️ 与 reasoning delta 合并 |

---

## 7. 架构核心评价

### 7.1 设计亮点

1. **枚举派发(enum dispatch)架构清晰**: `AdapterKind` 枚举 + `dispatcher.rs` 的 macro 系统使得添加新 provider 非常简单
2. **抽象层设计合理**: `ChatOptions` → 各 adapter 自行提取需要的字段，不支持的静默忽略
3. **流式抽象统一**: `InterStreamEvent` 内部中间层 + `ChatStreamEvent` 对外接口，provider 事件差异在 adapter 内部消化
4. **模型名推断 + 命名空间**: `claude-*` → Anthropic, `gpt-*` → OpenAI, `ollama_cloud::gemma` → Ollama Cloud，同时支持 `adapter::model` 语法强制指定
5. **reasoning_effort 的三级映射**: Anthropic adapter 对不同的 Claude 模型版本使用不同的 thinking API 路径（effort / adaptive / legacy），这是最复杂的部分也做得很好
6. **OpenAI Resp 的 stateful session 支持**: `ChatRequest.previous_response_id` + `store` + `ChatResponse.response_id` 完整支持了 Responses API 的多轮特性

### 7.2 主要不足

1. **大量 provider 独有参数缺失**: `frequency_penalty`, `presence_penalty`, `logit_bias`, `top_k`, `n` 等在 genai 抽象层根本没有定义，无法使用
2. **Anthropic 的块级缓存有损**: 24h TTL 被降级为 1h，request-level cache_control 被忽略
3. **OpenAI Resp 的内置工具生态未暴露**: file_search, code_interpreter, image_generation, computer, MCP 等仅 web_search 有部分支持
4. **内容类型支持不完整**: Anthropic 不支持 图片 URL，OpenAI Resp 不支持音频输入，大部分 content part 类型在消息构建时被静默跳过
5. **extra_body 仅 OpenAI 兼容层支持**：Anthropic 没有提供低层逃逸，无法添加未被抽象的参数
6. **部分功能标记为 FIXME/TODO**: WebSearchConfig 的 domain filter、audio in responses 等尚未完成

### 7.3 适用场景

| 场景 | 推荐度 | 原因 |
|---|---|---|
| 基本聊天 + 多 provider 切换 | ✅ 优秀 | genai 的核心优势 |
| 工具调用 + 流式 | ✅ 良好 | 三者流式事件差异处理得当 |
| Structured Output (JSON Schema) | ✅ 良好 | 三者均支持，Genesis/Vertex 有额外兼容处理 |
| 多模态图片理解 | ⚠️ 可用 | Anthropic 图片 URL 不支持，需用 base64 |
| 音频输入/输出 | ⚠️ 仅 OpenAI Chat | 其他 adapter 不支持 |
| Prompt 缓存 | ⚠️ 有损 | Anthropic 块级→OpenAI 请求级, TTL 降级 |
| 高级推理控制 | ⚠️ 可用 | reasoning_effort 对 Anthropic 有复杂映射但仍缺失部分控制 |
| OpenAI Responses 全功能 | ❌ 不适合 | 大量内置工具/MCP 未暴露，去掉 Chat Completions 直接用意义不大 |
| 完整 provider 功能覆盖 | ❌ 不适合 | 这不是 genai 的设计目标 |

---

## 8. 与你之前的 API 对比报告的对照

| 之前识别的差异 | genai 处理方式 |
|---|---|
| `temperature` 范围差异（0-1 vs 0-2） | ⚠️ 直接透传，未做范围截断 |
| `max_tokens` vs `max_completion_tokens` | ✅ 按模型类型自动选择字段名 |
| `system` 参数结构差异 | ✅ 分别处理：Anthropic top-level, Chat inline, Resp 双写 |
| `tool_use`/`tool_result` vs `tool` role | ✅ 三种格式分别正确构建 |
| block-level vs request-level 缓存 | ⚠️ 架起了桥梁但有损 |
| thinking vs reasoning_effort | ✅ 三级映射处理 |
| Anthropic `top_k` | ❌ 未在 genai 抽象层定义 |
| Chat `frequency_penalty` 等 | ❌ 未在 genai 抽象层定义 |
| Responses 有状态会话 | ✅ 通过 `previous_response_id` + `store` 支持 |
| Responses 内置工具生态 | ❌ 几乎完全未暴露 |
| 流式事件格式不兼容 | ✅ 三种格式各自正确解析 |

---

*报告生成时间: 2026年5月*

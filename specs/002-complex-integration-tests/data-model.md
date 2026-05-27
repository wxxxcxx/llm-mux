# 数据模型: 复杂场景集成测试

**Feature**: 002-complex-integration-tests | **Date**: 2026-05-27

本功能不涉及持久化数据或数据库实体。以下为测试框架中的核心逻辑构造。

## 测试用例 (TestCase)

每个集成测试的抽象结构：

| 字段 | 类型 | 说明 |
|------|------|------|
| name | string | 测试名称，如 `test_empty_request_body` |
| user_story | enum | 所属用户故事 (US1-US6) |
| inbound_protocol | Protocol | 入站请求协议 (OpenAiChat / Anthropic) |
| target_protocol | Protocol | 后端目标协议 (OpenAiChat / Anthropic) |
| inbound_body | JSON | 发送到网关的请求体 |
| expected_status | u16 | 期望的 HTTP 状态码 |
| assertions | Vec<Assertion> | 对响应体的验证规则集合 |
| route_config | RouteConfig | 此测试专用的路由规则（可选，默认使用基础配置） |

## 断言 (Assertion)

| 字段 | 类型 | 说明 |
|------|------|------|
| path | string | JSON 路径，如 `choices[0].message.content` |
| operator | enum | 比较操作: `equals`, `contains`, `not_empty`, `present`, `matches` |
| expected | Value | 期望值（`not_empty` 和 `present` 无需 expected） |

## 测试配置 (TestConfig)

可编程构建的网关配置：

| 字段 | 类型 | 说明 |
|------|------|------|
| providers | Map<String, ProviderConfig> | provider 定义 |
| routes | Vec<RouteRule> | 路由规则列表 |

Provider 和 RouteRule 类型复用 `llm-mux-core` 中已定义的结构，不重复定义。

## SSE 状态机 (SseStateMachine)

流式测试中验证事件序列的状态机：

```
初始状态
  │
  ├─ data:{message_start} ──▶ 已开始
  │
  ├─ data:{content_block_start} ──▶ 内容块已开始
  │     │
  │     ├─ data:{content_block_delta} ──▶ 内容块进行中
  │     │     │
  │     │     └─ data:{content_block_delta} ──▶ 内容块进行中 (循环)
  │     │
  │     └─ data:{content_block_stop} ──▶ 内容块已结束
  │
  ├─ data:{message_delta} ──▶ 消息增量
  │
  └─ data:{message_stop} ──▶ 消息结束 ──▶ data:[DONE] ──▶ 流结束 ✅
```

| 状态 | 允许的下一个事件类型 |
|------|---------------------|
| 初始 | message_start |
| message_start | content_block_start |
| content_block_start | content_block_delta |
| content_block_delta | content_block_delta, content_block_stop |
| content_block_stop | content_block_start (多块), message_delta |
| message_delta | message_stop |
| message_stop | [DONE] |
| [DONE] | 流结束 (EOF) |

## 测试数据

| 数据集 | 内容 | 用途 |
|--------|------|------|
| unicode_cases | 中文、emoji、阿拉伯文、零宽字符 | 特殊字符透传测试 |
| injection_payloads | HTML 标签、SQL 片段、script 注入 | 注入透传测试 |
| large_payloads | 10KB / 100KB / 1MB 消息体 | 消息大小限制测试 |
| multi_turn_tool_calls | 3 轮 tool_use ↔ tool_result 对话 | 多轮工具调用完整性测试 |
| system_prompt_blocks | 1-3 段 system text blocks | system prompt 合并测试 |
| model_names | 有效/无效/空/通配符模型名 | 路由匹配测试 |

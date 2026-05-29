# Investigation: Epic 1 实现覆盖验证

## Hand-off Brief

1. **调查目标：** 验证 Epic 1 中定义的 6 个故事是否在代码中完整实现
2. **当前状态：** 全部 6 个故事的核心功能已实现，1 项部分覆盖
3. **下一步：** 审查具体发现后决定是否需要补充遗漏的测试

## Case Info

| Field | Value |
|-------|-------|
| Ticket | epic1-verification |
| Date opened | 2026-05-29 |
| Status | Active |
| System | llm-mux Rust workspace（5 crates） |
| Evidence sources | Source code, test files, Cargo.toml, Dockerfile |

## Story 1.1: 入站协议编解码

| AC | 证据 | 等级 |
|----|------|------|
| OpenAI Chat 请求解码 | `crates/adapters/openai/src/chat/mod.rs:27-148` — `Codec::decode_request()` | ✅ Confirmed |
| Anthropic Messages 请求解码 | `crates/adapters/anthropic/src/messages/mod.rs:27-148` | ✅ Confirmed |
| OpenAI Responses 请求解码 | `crates/adapters/openai-resp/src/responses/mod.rs` | ✅ Confirmed |
| 响应编码回原始协议 | `crates/adapters/openai/src/chat/encode.rs:7-82` — stop_reason 映射完整 | ✅ Confirmed |
| 协议路径自动识别 | `crates/gateway/src/handlers/mod.rs:55-93` — 3 个端点函数 | ✅ Confirmed |

**测试覆盖：**
- OpenAI 单元测试 4 个: `crates/adapters/openai/tests/chat_tests.rs`
- Anthropic 单元测试 6 个: `crates/adapters/anthropic/tests/messages_tests.rs`
- OpenAI Responses 测试: **无** ⚠️
- 跨协议测试 4 个: `crates/gateway/tests/cross_protocol_tests.rs`

## Story 1.2: 可配置路由

| AC | 证据 | 等级 |
|----|------|------|
| 多条件 AND 匹配 | `crates/core/src/codec.rs:250-321` — Router::route() | ✅ Confirmed |
| 通配符匹配 | `codec.rs:217-247` — wildcard_match() 支持 `*`/`?` | ✅ Confirmed |
| 模型名映射 | `codec.rs:288-300` — 精确+通配符映射 | ✅ Confirmed |
| 配置校验 | `crates/gateway/src/config.rs:114-144` — validate() | ✅ Confirmed |
| 兜底规则约束 | `config.rs:132-143` — 兜底必须无条件 | ✅ Confirmed |

## Story 1.3: 流式 SSE 代理

| AC | 证据 | 等级 |
|----|------|------|
| 逐事件翻译 | `crates/gateway/src/handlers/mod.rs:426-481` — genai_stream_to_sse() | ✅ Confirmed |
| 非缓冲 | `handlers/mod.rs:440` — `futures::StreamExt::next()` 逐事件 | ✅ Confirmed |
| OpenAI Chat SSE 格式 | `crates/adapters/openai/src/chat/adapter.rs:61-70` — `data: [DONE]` | ✅ Confirmed |
| Anthropic SSE 格式 | `handlers/mod.rs:456-464` — 自动注入 message_start/block_start | ✅ Confirmed |
| Anthropic 流事件 | `handlers/mod.rs:467-470` — content_block_delta/message_stop | ✅ Confirmed |
| Responses SSE 格式 | `handlers/mod.rs:471-474` — response.output_text.delta | ✅ Confirmed |
| 流中断错误 | `handlers/mod.rs:443-444` — SSE error 事件 | ✅ Confirmed |

**测试覆盖：**
- SSE 验证辅助函数: `crates/gateway/tests/streaming_tests.rs:118-189`
- 流式集成测试（标注 "to be implemented"）: `streaming_tests.rs:191-197` ⚠️ 仅框架，AC 测试尚待实现

## Story 1.4: CLI 与部署

| AC | 证据 | 等级 |
|----|------|------|
| CLI start 命令 | `crates/gateway/src/main.rs:243-314` — port/host/config/log-level/daemon/pid | ✅ Confirmed |
| CLI stop 命令 | `main.rs:208-241` — PID 文件管理 + 优雅+强制终止 | ✅ Confirmed |
| CLI config 子命令 | `main.rs:168-207` — init/validate/show | ✅ Confirmed |
| Docker 构建 | `Dockerfile` — 多阶段 rust:1.85→distroless/cc | ✅ Confirmed |
| 健康检查 | `crates/gateway/src/handlers/mod.rs:46-53` — `GET /health` | ✅ Confirmed |
| 优雅关闭 | `crates/gateway/src/server.rs:60-78` — SIGTERM/Ctrl-C + drain | ✅ Confirmed |

## Story 1.5: 认证与安全

| AC | 证据 | 等级 |
|----|------|------|
| API Key 白名单 | `crates/core/src/codec.rs:162-183` — ConfigAuthenticator | ✅ Confirmed |
| 认证拦截 | `crates/gateway/src/handlers/mod.rs:24-36` — validate_auth() → 401 | ✅ Confirmed |
| 三种协议 Key 提取 | `handlers/mod.rs:95-118` — extract_api_key_chat/anthropic | ✅ Confirmed |
| 空列表不验证 | `codec.rs:177` — `self.keys.is_empty() → Ok(())` | ✅ Confirmed |
| Auth 测试 | `crates/gateway/tests/e2e_test.rs:7-20` — valid/invalid key | ✅ Confirmed |

## Story 1.6: 可观测性基础

| AC | 证据 | 等级 |
|----|------|------|
| UUID v7 Request ID | `crates/gateway/src/middleware.rs:8` — `Uuid::now_v7()` | ✅ Confirmed |
| X-Request-ID 响应头 | `middleware.rs:16-18` — HeaderValue 注入 | ✅ Confirmed |
| tracing span | `middleware.rs:12` — `info_span!("request", request_id)` | ✅ Confirmed |
| 结构化日志 | `crates/gateway/src/lib.rs:17-31` — json/compact + env-filter | ✅ Confirmed |
| 日志级别可配置 | `crates/gateway/src/config.rs:66` — log_level | ✅ Confirmed |

## Summary

| Story | 状态 | AC 覆盖 |
|-------|------|---------|
| 1.1 入站编解码 | ✅ 已实现 | 5/5 AC Confirmed, openai-resp 缺测试 ⚠️ |
| 1.2 可配置路由 | ✅ 已实现 | 5/5 AC Confirmed |
| 1.3 流式 SSE | ✅ 已实现 | 7/7 AC Confirmed, 流式集成测试待完善 ⚠️ |
| 1.4 CLI/部署 | ✅ 已实现 | 6/6 AC Confirmed |
| 1.5 认证 | ✅ 已实现 | 5/5 AC Confirmed |
| 1.6 可观测 | ✅ 已实现 | 5/5 AC Confirmed |

### 发现的不一致项

1. ⚠️ **openai-resp 适配器缺少测试文件** — `crates/adapters/openai-resp/tests/` 不存在
2. ⚠️ **流式集成测试未完成** — `crates/gateway/tests/streaming_tests.rs:191-197` 注释标注 "to be implemented in Phase 6/7"，SSE 事件序列验证的完整测试尚未编写

## Conclusion

**Confidence: High**

Epic 1 的全部 6 个故事的核心功能已在代码中实现，AC 覆盖 33/33 条 Confirmed。两处改进机会：
1. openai-resp 适配器补充单元测试
2. 补全流式集成测试的完整 AC 验证

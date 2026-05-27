# LLM Mux

高性能 LLM API 协议互转网关，通过统一 **Internal Representation (IR)** 将 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种协议双向互转。

## 架构

```
OpenAI Chat ──┐
OpenAI Resp ──┼── Codec ──▶ IR ──▶ Codec ──▶ Anthropic
Anthropic  ──┘                        └─▶ OpenAI Chat
                                       └─▶ OpenAI Resp
```

所有协议转换经过统一 IR 层，每个编解码器独立实现 `Codec` trait，新增协议无需修改核心逻辑。

## 特性

- 三协议双向互转（Chat Completions / Responses / Messages）
- 请求/响应/流式事件全覆盖
- 未知字段透传（provider_extensions）
- 可配置路由（模型名、协议、工具、媒体多条件匹配）
- 模型名映射（如 `gpt-4o` → `claude-sonnet-4-6`）
- 认证中间件（API Key 白名单）
- SSE 流式代理（带背压传播）
- 单一静态二进制，< 15MB

## 快速开始

```bash
# 生成配置
llm-mux config init

# 编辑配置（填入 API Key）
vim config.yaml

# 启动服务
llm-mux start

# 验证
curl http://localhost:8080/health

# 使用 OpenAI SDK 请求，自动路由到 Anthropic
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-xxx" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}'
```

## 配置

```yaml
host: "127.0.0.1"
port: 8080

providers:
  openai:
    protocol: openai-chat
    base_url: "https://api.openai.com/v1"
    api_key: "${OPENAI_API_KEY}"
    model_mapping:
      "gpt-4o": "claude-sonnet-4-6"

  anthropic:
    protocol: anthropic
    base_url: "https://api.anthropic.com"
    api_key: "${ANTHROPIC_API_KEY}"

routes:
  - models: ["claude-*"]
    provider: anthropic
  - models: ["*"]
    provider: openai
```

## 项目结构

```
llm-mux/
├── crates/
│   ├── llm-mux-core/           # IR 定义 + Codec trait + 路由/认证
│   ├── llm-mux-gateway/        # HTTP 服务 + CLI + SSE 代理
│   └── llm-mux-codecs/
│       ├── openai-chat/        # OpenAI Chat Completions 编解码
│       ├── anthropic/          # Anthropic Messages 编解码
│       └── openai-responses/   # OpenAI Responses 编解码
├── specs/                      # 功能规格文档
├── config.example.yaml
└── Dockerfile
```

## 构建

```bash
cargo build --release
# target/release/llm-mux
```

## 性能目标

| 指标 | 目标 |
|------|------|
| 请求协议转换 | ≤ 1ms（典型 < 10KB body） |
| 流事件转换 | ≤ 100μs/event |
| 首 token 延迟增量 | ≤ 100ms |
| 100 并发 p95 延迟 | ≤ 2ms |
| 内存占用 | ≤ 50MB |
| 二进制体积 | < 15MB (stripped) |

## 协议

MIT

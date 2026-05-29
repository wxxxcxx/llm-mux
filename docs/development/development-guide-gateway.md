# 开发指南 — Gateway

## 环境要求

- **Rust** 1.85+（edition 2021）
- Cargo（随 Rust 安装）
- （可选）Docker

## 快速开始

```bash
# 克隆并进入项目
cd llm-mux

# 构建
cargo build

# 生成默认配置
cargo run -- config init

# 编辑配置（填入 API Key）
# vim config.yaml

# 启动服务
cargo run -- start

# 验证
curl http://localhost:8080/health
```

## 常用命令

```bash
# 构建（release，优化体积）
cargo build --release

# 运行所有测试
cargo test

# 运行特定 crate 测试
cargo test -p llm-mux-core
cargo test -p llm-mux-gateway
cargo test -p openai-chat-codec
cargo test -p anthropic-codec
cargo test -p openai-responses-codec

# 运行集成测试
cargo test --features integration

# 运行 clippy
cargo clippy --workspace -- -D warnings

# 检查代码格式
cargo fmt --check

# 生成文档
cargo doc --no-deps --open
```

## CLI 命令

| 命令 | 说明 |
|------|------|
| `llm-mux start` | 启动网关服务 |
| `llm-mux stop` | 停止运行中的网关 |
| `llm-mux config init` | 生成默认配置文件 |
| `llm-mux config validate` | 校验配置文件 |
| `llm-mux config show` | 展示当前配置 |

### start 参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--port` | 8080 | 监听端口 |
| `--host` | 127.0.0.1 | 监听地址 |
| `--config` | config.yaml | 配置文件路径 |
| `--log-level` | info | 日志级别 |
| `--daemon` | false | 以守护进程方式运行 |
| `--pid-file` | — | PID 文件路径 |

## 项目约定

- 每个 crate 有独立的 `tests/` 目录
- 测试使用内嵌 JSON 字符串，无需 fixture 文件
- 解码测试直接传字节切片调用 `decode_request(body.as_bytes())`
- 编码测试构造 IR struct 后调用 `encode_response`，反序列化 JSON 断言

# 快速开始: LLM Mux

**Feature**: LLM Mux 三协议互转网关 | **Date**: 2026-05-27

## 前置条件

- [Rust](https://rustup.rs/) 1.70+ (stable)
- 任意 OpenAI 或 Anthropic API Key

## 下载 & 运行

### 方式一: 预编译二进制

```bash
# Linux x86_64
curl -LO https://github.com/W/llm-mux/releases/latest/download/llm-mux-x86_64-unknown-linux-gnu
chmod +x llm-mux-x86_64-unknown-linux-gnu

# 生成默认配置
./llm-mux-x86_64-unknown-linux-gnu config init

# 编辑配置（填入 API Key 和路由规则）
vim config.yaml

# 启动服务（前台）
./llm-mux-x86_64-unknown-linux-gnu start

# 或启动为守护进程
./llm-mux-x86_64-unknown-linux-gnu start --daemon
```

### 方式二: Docker

```bash
docker run -p 8080:8080 \
  -e OPENAI_API_KEY=sk-xxx \
  -v $(pwd)/config.yaml:/config.yaml \
  ghcr.io/W/llm-mux:latest
```

### 方式三: 源码编译

```bash
git clone https://github.com/W/llm-mux
cd llm-mux
cargo build --release

# 二进制在 target/release/llm-mux
cp target/release/llm-mux /usr/local/bin/
```

### 方式四: Rust 库嵌入

```toml
# Cargo.toml
[dependencies]
llm-mux-core = "0.1"
llm-mux-gateway = "0.1"
```

```rust
use llm_mux_gateway::Server;

#[tokio::main]
async fn main() {
    let config = llm_mux_server::Config::from_file("config.yaml").unwrap();
    let server = Server::new(config);
    server.serve().await.unwrap();
}
```

## 验证

```bash
# 健康检查
curl http://localhost:8080/health
# → {"status":"ok"}

# 使用 OpenAI SDK 发起请求
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-xxx" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
# → {"id":"...","choices":[...],...}
```

## 下一步

- 配置多路由: 详见 [CLI 契约](./contracts/cli.md)
- 协议转换: 请求 OpenAI 格式自动转为 Anthropic 格式路由至对应后端
- 流式支持: 添加 `"stream": true` 即可获得 SSE 流式响应

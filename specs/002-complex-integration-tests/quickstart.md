# 快速开始: 复杂场景集成测试

**Feature**: 002-complex-integration-tests | **Date**: 2026-05-27

## 前置条件

- Rust 1.70+ 工具链 (`rustup`)
- 有效的 opencode.go API Key（配置在 `.env` 文件中）
- 网络连接（测试调用真实后端 API）

## 环境配置

```bash
# 项目根目录下创建 .env 文件（已在 .gitignore 中）
echo 'OPENCODE_API_KEY=sk-xxxx' > .env
```

## 运行测试

```bash
# 运行全部 e2e 测试（含网络调用，约 3-5 分钟）
cargo test --test e2e_test -- --nocapture

# 仅运行边界条件测试
cargo test --test e2e_test boundary -- --nocapture

# 仅运行协议转换测试
cargo test --test e2e_test conversion -- --nocapture

# 仅运行异常容错测试
cargo test --test e2e_test error -- --nocapture

# 仅运行流式测试
cargo test --test streaming_tests -- --nocapture

# 跳过需要网络的测试（仅运行单元测试）
cargo test --lib
```

## 测试文件说明

| 文件 | 内容 | 网络依赖 |
|------|------|---------|
| `crates/llm-mux-gateway/tests/e2e_test.rs` | 基础连通性、协议转换、边界条件、路由、容错 | 有 |
| `crates/llm-mux-gateway/tests/cross_protocol_tests.rs` | Codec 编解码层往返测试 | 无 |
| `crates/llm-mux-gateway/tests/streaming_tests.rs` | 流式事件序列、SSE 格式合规 | 有 |
| `crates/llm-mux-codecs/openai-chat/tests/chat_tests.rs` | OpenAI Chat 编解码单元测试 | 无 |
| `crates/llm-mux-codecs/anthropic/tests/messages_tests.rs` | Anthropic Messages 编解码单元测试 | 无 |

## 测试架构

```
测试用例
  ├── load_config()     # 读取 .env → 构建 Config (可定制路由)
  ├── build_app()       # Config → Router + AppState
  ├── start_server()    # Router + TcpListener:0 → 随机端口服务
  ├── reqwest::Client   # HTTP 客户端发送请求
  └── assert!()         # 验证状态码 / JSON 字段 / SSE 序列
```

每个测试独立启动服务器实例，绑定随机端口，无状态共享。

## 添加新测试

```rust
#[tokio::test]
async fn test_your_scenario() {
    let (addr, api_key) = start_server().await;

    let body = json!({
        "model": "your-model",
        "messages": [{"role": "user", "content": "your prompt"}],
        "max_tokens": 50,
    });

    let resp = client()
        .post(format!("http://{}/v1/chat/completions", addr))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .unwrap();

    let text = resp.text().await.unwrap();
    let data: serde_json::Value = serde_json::from_str(&text).unwrap();

    // 验证响应结构
    assert!(data["choices"].is_array());
}
```

# 技术调研: 复杂场景集成测试

**Feature**: 002-complex-integration-tests | **Date**: 2026-05-27

## 1. 测试服务器生命周期管理

### Decision: 每个测试独立启动 axum 服务器，绑定随机端口

**Rationale**:
- `tokio::test` 每个测试在独立 tokio runtime 中运行，需要各自的服务实例
- 使用 `TcpListener::bind("127.0.0.1:0")` 获取随机端口，避免端口冲突
- 服务器在 `tokio::spawn` 中运行，测试结束时随 tokio runtime 清理自动释放
- 每个测试构建独立的 Config / Router / AppState，不共享状态

**Alternatives considered**:
- 全测试共享一个服务器实例：端口固定、串行执行限制、测试间状态污染风险 → 不采用
- `axum::test::TestServer`：仅支持模拟请求，无法验证实际 TCP 通信 → 不采用
- 子进程启动 `llm-mux` 二进制：启动成本高（编译 + fork），调试困难 → 不采用

### Current Pattern (from e2e_test.rs):

```rust
async fn start_server() -> (SocketAddr, String) {
    let (config, api_key) = load_config();
    let app = build_app(config);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    tokio::time::sleep(Duration::from_millis(100)).await;
    (addr, api_key)
}
```

## 2. 错误场景模拟策略

### Decision: 通过定制 Config 路由到不存在的后端来模拟错误

**Rationale**:
- 网关本身不维护 mock 后端，需要通过配置来触发期望的错误行为
- 各类错误场景的模拟方式：

| 错误场景 | 模拟方式 |
|---------|---------|
| 后端 HTTP 4xx | 路由到真实后端但使用无效 model 名称 |
| 后端 HTTP 5xx | 路由到不存在的 base_url（如 `http://127.0.0.1:1/`）→ 连接被拒 |
| 连接超时 | 路由到不可达 IP（如 `http://10.255.255.1/`）→ TCP SYN 超时 |
| 非 JSON 响应 | 路由到返回 HTML 的 HTTP 服务（如 `http://httpbin.org/html`） |
| 无效 JSON 请求体 | 直接发送 `"not json"` 字符串到 `/v1/chat/completions` |
| 流中断 | 建议后端流式过程中关闭连接（模拟难度高，考虑使用 mock） |

**Alternatives considered**:
- 使用 `mockito` 或 `wiremock`：需要额外依赖，且无法覆盖真实网络层行为 → 部分采用（流中断场景）
- 仅做单元测试：无法验证 HTTP 层行为 → 不采用

## 3. SSE 流格式验证策略

### Decision: 逐行读取 SSE 响应流，使用状态机验证事件序列

**Rationale**:
- SSE 规范要求 `data:` 前缀、空行分隔事件、`[DONE]` 终止
- 需要验证事件序列的顺序（content_block_start → delta → content_block_stop → message_stop）
- 不需要解析每个事件的 JSON 内容（编解码层已有单元测试覆盖），但需验证：
  1. 每行以 `data:` 开头
  2. 事件间有空行分隔
  3. 流以 `data: [DONE]` 结束
  4. 多行 JSON 不被分割

**Implementation approach**:
```rust
async fn validate_sse_stream(resp: Response) {
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut event_count = 0;
    let mut saw_done = false;
    // ...逐 chunk 读取，按 \n\n 分割事件
    assert!(saw_done, "SSE stream must end with [DONE]");
}
```

**Alternatives considered**:
- 使用 `eventsource-stream` crate：额外依赖，不够灵活 → 不采用
- 仅检查最后一个事件：无法验证序列完整性 → 不采用

## 4. 并发测试策略

### Decision: 使用 `tokio::spawn` + `futures::join_all` 实现并发请求测试

**Rationale**:
- `tokio::test` 已在多线程 runtime 中运行
- `futures::join_all` 等待所有并发请求完成
- 需验证：各请求结果独立（无数据串扰）、路由各自独立匹配

**Implementation pattern**:
```rust
#[tokio::test]
async fn test_concurrent_requests() {
    let (addr, api_key) = start_server().await;
    let handles: Vec<_> = (0..10).map(|i| {
        let addr = addr;
        let api_key = api_key.clone();
        tokio::spawn(async move {
            // send request with unique model/content
        })
    }).collect();
    let results = futures::future::join_all(handles).await;
    for r in results {
        assert!(r.unwrap().unwrap().status().is_success());
    }
}
```

**Constraints**: `futures` 已在 workspace dependencies 中（`futures = { version = "0.3", default-features = false, features = ["std"] }`），无需新增依赖。

## 5. 测试配置管理

### Decision: 测试内嵌 YAML 配置，通过函数参数定制路由规则

**Rationale**:
- 不同测试需要不同的路由规则（触发不同协议转换方向、测试 model_mapping 等）
- 使用函数式 API 构建 Config 对象：基础配置固定，路由部分可覆盖
- 避免创建多个 `.yaml` 配置文件

**Implementation**:
```rust
fn test_config(routes: &str) -> Config {
    let base = format!(r#"host: "127.0.0.1"\nport: 0\n...{routes}"#, ...);
    serde_yaml::from_str(&base).unwrap()
}
```

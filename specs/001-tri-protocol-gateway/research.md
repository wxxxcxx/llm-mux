# 技术调研: LLM Mux 三协议互转网关

**Created**: 2026-05-27 | **Feature**: [spec.md](./spec.md)

## 决策记录

### 1. HTTP 框架: axum

- **决策**: 使用 `axum` 0.7+ 配合 `tokio` 异步运行时
- **理由**:
  - constitution 已指定 axum 为 HTTP 框架
  - 基于 tower 中间件生态，与 tracing 原生集成
  - Streaming body 支持良好，适合 SSE 流式代理场景
  - 类型安全的 extractor/response 体系，减少运行时错误
- **备选方案**:
  - `actix-web`: 生态成熟但依赖自有运行时，与 tokio 生态集成成本高
  - `warp`: Filter 组合子模式学习曲线陡峭，社区活跃度下降
  - `hyper` 直接使用: 过于底层，缺少路由/中间件抽象

### 2. CLI 框架: clap + 子命令模式

- **决策**: 使用 `clap` 4.x derive 模式，采用 `start` / `stop` / `config` 子命令结构
- **理由**:
  - Rust 生态 CLI 标准库，社区活跃
  - Derive macro 减少样板代码，类型安全
  - 子命令模式天然支持不同操作（启动/停止/配置管理），符合老派守护进程习惯
  - `--daemon` 参数使用 PID 文件 + fork 模式（Unix）或直接启动（Windows/macOS）
- **子命令结构**:
  ```
  llm-mux start    [--port] [--host] [--config] [--log-level] [--daemon]
  llm-mux stop     [--pid-file]
  llm-mux config   {init|validate|show} [--path]
  ```
- **备选方案**:
  - 纯标志模式（`--port 8080` 直接启动）: 简单但不支持服务管理
  - systemd/launchd 集成: 过度依赖平台，首版不引入

### 3. 配置格式: YAML

- **决策**: 使用 `serde_yaml` 解析 YAML 配置文件
- **理由**:
  - 配置文件（路由规则、后端凭证）嵌套结构复杂，YAML 可读性好
  - serde 集成零成本，与现有类型系统无缝对接
  - spec 已明确指定 YAML 格式
- **备选方案**:
  - TOML: 嵌套支持弱于 YAML
  - JSON: 可读性差，无注释支持

### 4. 日志系统: tracing-subscriber

- **决策**: 使用 `tracing` + `tracing-subscriber`，默认 JSON 格式输出到 stdout
- **理由**:
  - `tracing` 已是 workspace 依赖
  - 结构化日志天然支持 JSON，便于日志采集管道解析
  - Span 机制适合请求级追踪（每个请求一个 span，携带 request_id/model/latency）
  - `--log-level` 通过 `RUST_LOG` 环境变量或 `EnvFilter` 一致实现
- **备选方案**:
  - `log` + `env_logger`: 缺少结构化字段支持
  - `slog`: 生态较小

### 5. Docker 基础镜像: distroless (cc)

- **决策**: 使用 `gcr.io/distroless/cc-debian12` 非 root 运行
- **理由**:
  - 包含 glibc 和 libstdc++，兼容 Rust 静态链接
  - 不含 shell/包管理器，攻击面极小
  - 镜像大小 < 20MB（含二进制 < 15MB，总镜像 ~25MB）
  - 非 root 运行符合安全最佳实践
- **备选方案**:
  - `scratch`: 完全空镜像，更小但需要 musl 静态链接（需 rustup target add x86_64-unknown-linux-musl）
  - `alpine`: 包含包管理器，攻击面更大

### 6. SSE 流式代理架构

- **决策**: axum streaming body + tokio mpsc channel 逐事件转发
- **设计**:
  ```
  Client --[SSE]--> axum handler --[req]--> downstream HTTP client (reqwest)
                                                      |
  Client <--[SSE]-- axum stream body <--[evt]-- tokio mpsc channel
  ```
  - 后端 SSE 流逐事件解析 → `IrStreamEvent` → 协议转换 → 客户端 SSE 格式
  - `tokio::sync::mpsc` 提供背压传播（channel bounded）
  - 不缓冲完整响应体
- **理由**:
  - 满足 FR-013 逐事件翻译 + 零缓冲要求
  - 满足 constitution IV 背压传播要求
  - axum 原生支持 `Stream<Item = Result<Bytes, ...>>` 作为 response body

### 7. 请求 ID 生成: UUID v7

- **决策**: 使用 UUID v7（时间排序 + 随机后缀）生成 Request ID
- **理由**:
  - 时间排序便于日志检索（按时间顺序）
  - 随机后缀避免碰撞
  - `uuid` crate 成熟稳定
- **备选方案**:
  - UUID v4: 纯随机，无时间排序
  - ULID: 功能相似，但 UUID v7 标准化程度更高
  - Snowflake: 需要协调节点 ID，不适合单进程场景

### 8. OpenAI Responses 协议模型设计

- **决策**: 在 `openai-responses` crate 中定义独立的请求/响应模型，与 Chat Completions 模型不共享
- **理由**:
  - Responses API 语义差异大：用 `instructions` 替代 `messages[system]`，内置 `tools` 在 response 级别声明
  - 状态管理（`conversation` 上下文）、输出类型（`text`/`json_schema`）与 Chat API 不同
  - 独立模型避免 Chat 模型被污染，保持各 codec 独立演变
- **关键映射挑战**:
  - `instructions` → `system_prompt` (encode) / `system_prompt` → `instructions` (decode)
  - `previous_response_id` → 无 IR 直接对应（需在 Router 层处理状态）
  - `text.format` (json_schema) → `IrResponseFormat`

### 9. 认证实现策略

- **决策**: 实现 `Authenticator` trait 的基于配置文件的 API Key 验证
- **设计**: 从 YAML 配置加载 `api_keys: [key1, key2, ...]` 列表，Authenticator 执行 O(1) HashSet 查找
- **理由**: 满足 FR-007 最小要求，与无状态定位一致
- **备选方案**:
  - 透传验证（不检查，转发给后端验证）: 简单但不满足 FR-007
  - JWT/OAuth: 过度设计

### 10. 二进制体积优化

- **决策**: 组合使用 LTO + strip + opt-level="z" + panic="abort"
- **配置**:
  ```toml
  [profile.release]
  lto = true
  codegen-units = 1
  opt-level = "z"
  strip = true
  panic = "abort"
  ```
- **预期**: < 15MB (Linux x86_64)，满足 SC-003

### 11. 路由实现策略

- **决策**: 实现 `ConfigurableRouter`，YAML 配置采用 **Provider 集中定义 + 路由引用 + 从上到下首个命中** 模式
- **两种匹配机制**:
  1. **Route 匹配** (决定走哪个 Provider): 基于 model/protocol/stream/has_tools/has_media 多条件 AND 组合，**从上到下首个命中生效**，兜底 `models: ["*"]` 写在最后
  2. **Model Mapping** (决定后端实际模型名): 路由命中后在 Provider 级别做模型名转换
- **设计**:
  ```yaml
  providers:
    anthropic:
      protocol: anthropic
      base_url: "https://api.anthropic.com"
      api_key: "${ANTHROPIC_API_KEY}"
      model_mapping:
        "gpt-4o": "claude-sonnet-4-6"
        "*": "claude-sonnet-4-6"
  routes:
    - models: ["gpt-4o"]         # 首个检查: 精确命中
      protocol: openai-chat
      stream: true
      provider: azure-gpt4
    - models: ["claude-*"]       # 次之: 通配
      provider: anthropic
    - models: ["*"]              # 兜底: 必须放在最后
      provider: openai
  ```
- **理由**:
  - 自上而下首个命中是最直观的匹配模型（大多数路由框架、nginx location 均采用此模式）
  - `protocol` 匹配允许按入站协议类型分流（如 Chat 请求走高速后端，Responses 请求走特定后端）
  - 兜底规则在最后确保所有请求都有归宿

## 未解决项

无。所有技术选型已明确，可直接进入 Phase 1 设计阶段。

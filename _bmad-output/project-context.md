---
project_name: 'llm-mux'
user_name: 'W'
date: '2026-05-29'
sections_completed:
  ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'quality_rules', 'workflow_rules', 'anti_patterns']
status: 'complete'
rule_count: 30
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

- **Language:** Rust edition 2021, workspace resolver = "2"
- **LLM Protocol Crate:** genai v0.6 (external, serves as unified IR + client)
- **Web Framework:** axum 0.7
- **Async Runtime:** tokio 1 (features = ["full"])
- **Serialization:** serde 1 + serde_json 1 + serde_yaml 0.9
- **CLI:** clap 4 (features = ["derive", "env"])
- **Logging:** tracing 0.1 + tracing-subscriber 0.3 (features = ["json", "env-filter"])
- **Error Handling:** thiserror 2
- **UUID:** uuid 1 (features = ["v4", "v7"])
- **Streaming:** futures 0.3, tokio-stream 0.1
- **Env:** dotenvy 0.15
- **Unix-only:** libc 0.2 (cfg gate)

### Build Configuration

- **Release profile:** LTO, codegen-units=1, opt-level="z", strip=true, panic="abort"
- **Lints:** `unsafe_code = "forbid"`, `missing_docs = "warn"`, `unused_qualifications = "warn"`
- **Clippy:** pedantic + nursery + cargo at warn level
  - Allowed: `module_name_repetitions`, `must_use_candidate`, `return_self_not_must_use`, `cast_possible_truncation`
- **Target platforms:** Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64
- **Binary size target:** < 15MB (stripped)

## Critical Implementation Rules

### Code Quality & Style Rules

- **文件行数限制：** 单个源文件不超过 400 行（含空行和注释），超出必须拆分为文件夹模块
- **方法行数限制：** 单个方法/函数不超过 80 行，超出需拆分为更小的方法
- **内部模块组织：** 所有内部子模块必须使用文件夹模块形式（`module_name/mod.rs` + 拆分文件），禁止使用单文件 `module_name.rs` 承载复杂子模块

### Language-Specific Rules

- **`unsafe_code = "forbid"`** — workspace 级别禁止所有 unsafe 代码（workspace lints.rust）
- **Error types** 统一使用 `thiserror` derive macro，派生 Debug + Display
- **公有项必须写 doc comment** — `missing_docs = "warn"`
- **Serde 枚举命名风格：** `#[serde(rename_all = "snake_case")]` 或 `"lowercase"`，视语义选择
- **Serde 可选字段：** 使用 `#[serde(skip_serializing_if = "Option::is_none")]`
- **Serde 集合字段：** 使用 `#[serde(default, skip_serializing_if = "Vec::is_empty")]` 或 `HashMap::is_empty`
- **Trait 对象约束：** 统一加 `Send + Sync`
- **不允许裸 `.unwrap()` / `.expect()`** — 使用 `?` 操作符或 `map_err` 转换

### Framework-Specific Rules

- **axum 路由：** 使用 `Router::new().route(...)` 链式构建，状态通过 `Arc<AppState>` + `.with_state()` 注入
- **axum 中间件：** 使用 `axum::middleware::from_fn`，放在 `layer()` 中
- **axum 优雅关闭：** Unix 用 SIGTERM，非 Unix 用 `signal::ctrl_c()`，通过 `with_graceful_shutdown` 接入
- **genai 集成：** `ChatRequest`/`ChatResponse`/`ChatStreamEvent` 作为统一 IR 类型
- **genai 客户端：** 使用 `genai::Client::default()` 创建，通过 `AdapterKind` 选择后端
- **Workspace 内部引用：** 所有内部 crate 在 `[workspace.dependencies]` 中声明，各 crate 用 `name.workspace = true` 引用
- **适配器 crate 命名：** `{vendor}-{protocol}-codec` 模式（如 `openai-chat-codec`、`anthropic-codec`）
- **配置加载：** 使用 `serde_yaml` + `dotenvy`（环境变量注入 `${VAR}` 模式）
- **日志初始化：** 使用 `tracing-subscriber` 的 json + env-filter 特性

### Testing Rules

- **测试框架：** 使用 `cargo test`，每个 crate 有独立的 `tests/` 目录存放集成测试
- **单元测试：** 在 `src/` 内联，使用 `#[cfg(test)] mod tests { ... }`
- **解码测试模式：** 直接传 JSON 字节切片调用 `decode_request(body.as_bytes())`，断言返回的 IR 字段
- **编码测试模式：** 构造 `IrResponse` / `IrRequest` struct，调用 `encode_response`，反序列化结果 JSON 后断言字段值
- **流事件测试：** 使用 `encode_stream_event` 测试每个事件类型的输出格式
- **集成测试：** 跨协议 roundtrip 测试在 `gateway/tests/`（`cross_protocol_tests.rs`、`e2e_test.rs`、`streaming_tests.rs`）
- **测试数据：** 直接在测试中嵌入 JSON 字符串，无需额外 fixture 文件

### Development Workflow Rules

- **`specs/` 目录即将移除** — 不要依赖 `specs/` 作为需求来源。需求/规格文档后续统一在 `_bmad-output/` 中管理
- **Git 忽略模式：** `target/`、`.vscode/`、`.idea/`、`.env`、`config.yaml` 不提交

### Critical Don't-Miss Rules

- **禁止 unsafe** — workspace 级别 `unsafe_code = "forbid"`，任何 unsafe 代码均不允许
- **禁止裸 unwrap/expect** — 错误处理必须使用 `?` 操作符或 `map_err` 转换
- **禁止跳过 missing_docs** — 所有 `pub` 项必须有 doc comment（`missing_docs = "warn"`）
- **热路径性能：** 流式事件转换（encode_stream_event）目标 ≤ 100μs/event，注意分配最小化
- **跨平台条件编译：** Unix-only 依赖（libc）必须使用 `#[cfg(unix)]` 条件 gate
- **Docker 构建：** 多阶段构建，builder 使用 `rust:1.85-slim-bookworm`，运行时使用 `gcr.io/distroless/cc-debian12`

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Update this file if new patterns emerge

**For Humans:**

- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

Last Updated: 2026-05-29

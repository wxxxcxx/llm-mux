# 实现计划: 基于 genai 适配器模式重构协议网关

**分支**: `003-genai-adapter-refactor` | **日期**: 2026-05-28 | **规格**: [spec.md](./spec.md)

**输入**: Feature specification from `specs/003-genai-adapter-refactor/spec.md`

## 概要

将 LLM Mux 从自建 IR + 自建 HTTP 客户端的架构，重构为以 genai crate 类型体系为统一 IR、genai Client 为下游调用层的适配器架构。核心变更：删除自定义 `IrRequest`/`IrResponse`/`IrStreamEvent` 及 `ContentBlock` 等类型，改为复用 genai 的 `ChatRequest`/`ChatResponse`/`ChatStreamEvent`/`ContentPart` 等类型；删除自建 reqwest HTTP 客户端，改为 genai `Client`；将现有 `Codec` trait 重构为 `Adapter` trait；Provider 配置使用 `format` 字段映射 genai `AdapterKind`。

## 技术上下文

| 项 | 值 |
|---|---|
| **Language/Version** | Rust edition 2021, workspace.resolver = "2" |
| **Primary Dependencies** | genai v0.6+（外部 crate）、serde + serde_json、axum、tokio |
| **Storage** | N/A（无状态网关） |
| **Testing** | cargo test（单元测试 per-crate + 集成测试 crossover） |
| **Target Platform** | Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64 |
| **Project Type** | Rust workspace（3 crates: core + gateway + adapters） |
| **Performance Goals** | 请求翻译 ≤1ms、流事件翻译 ≤100μs（同重构前） |
| **Constraints** | 二进制 < 15MB、no unsafe、无状态 |
| **Scale/Scope** | 3 种入站协议 × 25+ 种出站后端（genai AdapterKind） |

## 宪章合规检查

*门禁: 必须在阶段 0 研究前通过。阶段 1 设计后重新评估。*

### I. 代码质量与安全

- [x] `unsafe_code = "forbid"` — 重构后不变，genai 也不含 unsafe
- [x] clippy pedantic/nursery — 与 genai 的依赖引入无 lint 冲突
- [x] `thiserror` 错误类型 — Adapter trait 错误类型使用 `thiserror`
- [x] 无裸 `.unwrap()` / `.expect()` — 保持现有规范

### II. 测试优先

- [x] 跨协议 roundtrip 测试 — 保留并适配现有集成测试套件
- [x] 每个适配器功能需要对应测试 — FR-009 要求全量测试通过
- [x] 运行 `cargo test` 通过方可提交

### III. 跨协议保真度

- [x] 往返无损 — genai `ContentPart::Custom` + `ChatOptions.extra_body` 保证透传
- [x] 协议特有字段进入扩展机制 — 不在 IR 层丢弃
- [x] 响应编码对称于请求解码 — Adapter trait 两端方法对称
- [x] 注意：现有 `Converter` trait 对应新 Adapter trait；provider 逻辑仍在适配器中隔离

### IV. 性能设计

- [x] 流式逐事件处理 — genai 对三种协议的流式解析均已实现，无需自建
- [x] 热路径分配最小化 — 适配器纯数据转换，O(1) 每事件
- [x] 背压传播 — genai `ChatStream` 基于 `futures::Stream`，自然传播背压
- [x] 注意：HTTP 调用延迟由 genai Client 接管，适配器自身无网络开销

### V. Trait 驱动组合性

- [x] 核心抽象为 trait — `Adapter` trait 取代 `Codec` trait
- [x] 新 provider 不修改 core — 新增适配器 crate 实现 Adapter trait
- [x] IR 最小化 — genai 类型自身即为 IR，不再自定义
- [x] 注意：现有 `NoopConverter` 已被 genai 的透传能力取代

### 合规结论

所有 5 条宪章原则均合规。重构对齐了宪章精神：删除了自定义 IR（遵循 V-YAGNI），引入经过社区验证的 genai crate（遵循 I-II），利用 genai 的流式解析提升性能（遵循 IV），通过 Adapter trait 强化隔离（遵循 III-V）。

## 项目结构

### 文档（本功能）

```text
specs/003-genai-adapter-refactor/
├── plan.md              # 本文件
├── research.md          # 阶段 0: 技术调研
├── data-model.md        # 阶段 1: 数据模型
├── quickstart.md        # 阶段 1: 快速开始/迁移指南
├── contracts/           # 阶段 1: Adapter trait 合约
│   └── adapter-trait.md
└── tasks.md             # 阶段 2: /speckit.tasks 生成
```

### 源代码（仓库根目录）

```text
crates/
├── core/                       # 核心 crate（精简后）
│   └── src/
│       ├── lib.rs             # 重导出 genai 类型为 IR
│       ├── adapter.rs         # Adapter trait 定义（替代 codec.rs）
│       ├── types.rs           # Protocol 枚举、路由配置类型（精简）
│       └── error.rs           # 适配器错误类型
├── adapters/                   # 入站协议适配器（统一文件夹）
│   ├── openai/                # OpenAI Chat Completions 入站适配器
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── decode.rs      # 请求解码: ChatCompletionRequest → genai ChatRequest
│   │   │   └── encode.rs      # 响应编码: genai ChatResponse → OpenAI 响应体
│   │   └── tests/
│   ├── anthropic/             # Anthropic Messages 入站适配器
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── decode.rs
│   │   │   └── encode.rs
│   │   └── tests/
│   └── openai-resp/           # OpenAI Responses 入站适配器
│       ├── src/
│       │   ├── lib.rs
│       │   ├── decode.rs
│       │   └── encode.rs
│       └── tests/
└── gateway/                    # 网关 crate（大幅精简）
    ├── src/
    │   ├── main.rs            # CLI 入口
    │   ├── config.rs          # 配置加载（format 字段替代 protocol）
    │   ├── router.rs          # 路由匹配 + ServiceTarget 构造
    │   ├── server.rs          # Axum HTTP 路由
    │   ├── error.rs           # 错误映射: genai::Error → 协议格式
    │   └── client.rs          # genai Client 封装（可选，如单例管理）
    └── tests/
```

**结构决策**: 保持 workspace 分层架构。子 crate 去除 `llm-mux-` 前缀（`core`、`gateway`、`adapters/`）。适配器归入统一 `crates/adapters/` 文件夹管理。核心变化：
- Codec crates 改为 Adapter crates（职责从"双向编解码+HTTP"变为"入站解码+出站编码"，HTTP 职责移除）
- core 删除 `ir.rs`，新增 `adapter.rs`
- gateway 删除 reqwest 依赖，新增 genai 依赖
- 配置格式中 `protocol` 字段改为 `format`，值映射 genai `AdapterKind`

## 复杂度追踪

> 本条目的仅在宪章合规检查有违规需论证时填写。当前无违规。

无违规项。

## 阶段输出

| 阶段 | 输出 | 状态 |
|---|---|---|
| Phase 0 | `research.md` — 技术调研与决策 | 将在下文生成 |
| Phase 1 | `data-model.md` — 数据模型 | 将在下文生成 |
| Phase 1 | `contracts/adapter-trait.md` — Adapter trait 合约 | 将在下文生成 |
| Phase 1 | `quickstart.md` — 迁移指南 | 将在下文生成 |
| Phase 1 | `AGENTS.md` — 代理上下文更新 | 将在下文更新 |

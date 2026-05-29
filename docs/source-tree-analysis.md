# 源码树分析

```
llm-mux/                              # 根目录
├── Cargo.toml                        # Workspace 定义
├── Cargo.lock
├── config.example.yaml               # 配置示例
├── Dockerfile                        # 多阶段构建
├── README.md
├── AGENTS.md                         # AI Agent 指令
│
├── crates/
│   ├── core/                         # Part: core — 核心库
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs               # 公有 API 导出
│   │   │   ├── adapter.rs           # Adapter trait（旧层）
│   │   │   ├── codec.rs             # Codec trait + Router + Authenticator
│   │   │   ├── ir.rs                # IR 类型（IrRequest/IrResponse）
│   │   │   └── types.rs             # 基础类型（Protocol/ContentBlock/etc）
│   │   └── tests/
│   │       └── ir_tests.rs
│   │
│   ├── gateway/                      # Part: gateway — HTTP 服务 + CLI
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs              # CLI 入口（clap）
│   │   │   ├── lib.rs               # 库入口 + tracing 初始化
│   │   │   ├── config.rs            # 配置解析 + 校验
│   │   │   ├── server.rs            # axum 服务器 + 路由注册
│   │   │   ├── middleware.rs         # 请求 ID 中间件
│   │   │   └── handlers/
│   │   │       ├── mod.rs           # 路由处理 + 流式 SSE 代理
│   │   │       └── genai_bridge.rs  # genai 响应 → IR 转换
│   │   └── tests/
│   │       ├── e2e_test.rs
│   │       ├── cross_protocol_tests.rs
│   │       └── streaming_tests.rs
│   │
│   └── adapters/                     # Part: 协议适配器
│       ├── openai/                   # Part: openai-chat
│       │   ├── Cargo.toml
│       │   ├── src/
│       │   │   ├── lib.rs
│       │   │   ├── models.rs         # OpenAI Chat 模型定义
│       │   │   └── chat/
│       │   │       ├── mod.rs
│       │   │       ├── adapter.rs    # Adapter impl
│       │   │       ├── convert.rs    # IR ↔ 模型转换
│       │   │       └── encode.rs     # 流式事件编码
│       │   └── tests/
│       │       └── chat_tests.rs
│       │
│       ├── anthropic/                # Part: anthropic
│       │   ├── Cargo.toml
│       │   ├── src/
│       │   │   ├── lib.rs
│       │   │   ├── models.rs         # Anthropic 模型定义
│       │   │   └── messages/
│       │   │       ├── mod.rs
│       │   │       ├── adapter.rs    # Adapter impl
│       │   │       ├── convert.rs    # IR ↔ 模型转换
│       │   │       └── encode.rs     # 流式事件编码
│       │   └── tests/
│       │       └── messages_tests.rs
│       │
│       └── openai-resp/             # Part: openai-resp
│           ├── Cargo.toml
│           ├── src/
│           │   ├── lib.rs
│           │   ├── models.rs         # OpenAI Responses 模型定义
│           │   └── responses/
│           │       ├── mod.rs
│           │       ├── adapter.rs    # Adapter impl
│           │       ├── convert.rs    # IR ↔ 模型转换
│           │       └── encode.rs     # 流式事件编码
│           └── tests/
│
├── docs/                             # 项目知识库
│   ├── project-scan-report.json      # 文档扫描状态
│   ├── api-contracts-gateway.md
│   └── ...（其他文档）
│
├── _bmad-output/                     # BMad 产出
│   └── project-context.md
│
├── _bmad/                            # BMad 配置
├── specs/                            # 旧规格文档（即将移除）
└── tests/                            # 顶层集成测试

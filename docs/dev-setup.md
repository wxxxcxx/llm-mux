# macOS 开发环境备忘录

> 从 Windows (PowerShell) 迁移到 macOS (bash) 后的开发指引

## 环境

| 项目 | 值 |
|------|-----|
| 脚本引擎 | `sh` (bash) |
| AI 集成 | `opencode` |
| CLI 工具 | `specify` (v0.8.16.dev0) |

## 迁移记录

原项目在 Windows 上以 PowerShell (`script: ps`) 初始化。迁移到 macOS 后执行：

```bash
specify integration upgrade opencode --script sh --force
```

这会：将脚本类型切换为 bash、安装 `.specify/scripts/bash/` 下的脚本、更新 `integration.json` 和 `init-options.json`。

## 日常开发流程

```bash
# 1. 开新功能（创建 feature branch + spec 骨架）
specify workflow run speckit.specify "用中文描述功能"

# 2. 生成实现计划（基于 spec 生成 plan.md）
specify workflow run speckit.plan

# 3. 生成任务清单（基于 plan 生成 tasks.md）
specify workflow run speckit.tasks

# 4. 开始实现
#    - opencode 会根据 AGENTS.md 和 plan.md 提供上下文
#    - 直接与 opencode 对话完成编码

# 5. 验证
cargo test          # 运行测试
cargo clippy        # 代码检查
```

## 项目结构

```
llm-mux/
├── specs/                    # 功能规格目录
│   └── 001-tri-protocol-gateway/
│       ├── spec.md           # 功能规格
│       ├── plan.md           # 实现计划
│       ├── tasks.md          # 任务清单
│       ├── research.md       # 技术调研
│       ├── data-model.md     # 数据模型
│       ├── contracts/        # API 契约
│       └── quickstart.md     # 快速开始
├── crates/                   # Rust 源码
├── .specify/                 # spec-kit 配置
│   ├── scripts/bash/         # bash 脚本（macOS 使用）
│   └── scripts/powershell/   # 旧 powershell 脚本（保留备份）
└── docs/
    └── dev-setup.md          # 本文件
```

## 如果以后需要重建

```bash
# 重装 opencode 集成（会保留已修改文件）
specify integration upgrade opencode --script sh --force
```

## 注意

- `specify` 命令已安装到 `/Users/W/.local/bin/specify`
- 确保 `~/.local/bin` 在 `PATH` 中
- Rust 工具链通过 `rustup` 管理

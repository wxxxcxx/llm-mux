# LLM Mux — 项目文档索引

> 生成日期：2026-05-29

## 项目概览

- **类型：** Monorepo（5 parts）
- **主语言：** Rust 2021 edition
- **架构：** 适配器模式网关（无状态）

## 快速参考

| Part | 类型 | 技术栈 |
|------|------|--------|
| core | 库 | Rust, serde, genai |
| gateway | 服务 | Rust, axum, tokio, clap, genai |
| openai-chat | 适配器 | Rust, serde, genai |
| anthropic | 适配器 | Rust, serde, genai |
| openai-resp | 适配器 | Rust, serde, genai |

## 项目概览

- [Project Overview](./overview/project-overview.md)

## 架构设计

- [Architecture — Gateway](./architecture/architecture-gateway.md)
- [Source Tree Analysis](./source-tree-analysis.md)

## API 接口

- [API Contracts — Gateway](./api/api-contracts-gateway.md)

## 开发指南

- [Development Guide — Gateway](./development/development-guide-gateway.md)
- [Dev Setup](./development/dev-setup.md)

## 运维部署

- [Deployment Guide](./operations/deployment-guide.md)

## 外部协议参考

- [Anthropic Messages API](./reference/anthropic-messages-api.md)
- [OpenAI Chat Completions API](./reference/openai-chat-completions-api.md)
- [OpenAI Responses API](./reference/openai-responses-api.md)

## 技术调研

- [API Comparison Report](./research/api-comparison-report.md)
- [Rust genai Compatibility Report](./research/rust-genai-compatibility-report.md)
- [Reference Projects](./research/reference-projects.md)

## AI Agent 上下文

- [Project Context](../_bmad-output/project-context.md)

## 快速开始

```bash
cargo build --release
# 编辑 config.yaml
# ./target/release/llm-mux start
```

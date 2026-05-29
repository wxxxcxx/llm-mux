# 项目概览 — LLM Mux

## 用途

高性能 LLM API 协议互转网关。通过统一 IR 将 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种协议双向互转，单一二进制提供多协议接入能力。

## 技术栈摘要

| 类别 | 技术 |
|------|------|
| 语言 | Rust 2021 |
| Web | axum 0.7 |
| LLM | genai v0.6 |
| 异步 | tokio 1 |
| 构建 | Cargo workspace（5 crates） |

## 架构类型

**适配器模式网关** — 无状态、水平可扩展

## 仓库结构

**Monorepo** — 5 个 crate 的 Rust workspace

| Part | 路径 | 类型 | 说明 |
|------|------|------|------|
| core | `crates/core/` | 库 | IR 类型 + Codec trait + Router |
| gateway | `crates/gateway/` | 二进制 | HTTP 服务 + CLI |
| openai-chat | `crates/adapters/openai/` | 库 | OpenAI Chat 适配器 |
| anthropic | `crates/adapters/anthropic/` | 库 | Anthropic 适配器 |
| openai-resp | `crates/adapters/openai-resp/` | 库 | OpenAI Responses 适配器 |

## 性能目标

| 指标 | 目标 |
|------|------|
| 请求协议转换 | ≤ 1ms |
| 流事件转换 | ≤ 100μs/event |
| 二进制体积 | < 15MB |

## 文档索引

| 文档 | 说明 |
|------|------|
| [Architecture](./architecture-gateway.md) | 架构详情 |
| [API Contracts](./api-contracts-gateway.md) | 接口定义 |
| [Source Tree](./source-tree-analysis.md) | 源码结构 |
| [Development Guide](./development-guide-gateway.md) | 开发环境 |
| [Deployment Guide](./deployment-guide.md) | 部署运维 |

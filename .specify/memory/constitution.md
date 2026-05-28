<!--
Sync Impact Report
==================
Version change: 1.1.0 → 1.2.0 → 1.2.1
Previous amendments:
  - 1.0.0 (2026-05-27): Initial ratification — 5 Core Principles, Technical Constraints, Development Workflow, Governance
  - 1.1.0 (2026-05-27): Added "语言与本地化标准" section
  - 1.2.0 (2026-05-28): Added "代码规模控制 (Code Size Discipline)" as Principle VI
  - 1.2.1 (2026-05-28): Added directory-module organization sub-rule to Principle VI

Modified principles (1.2.1):
  - Principle VI: 新增目录模块组织规则

Added sections: None
Removed sections: None

Templates requiring updates:
  - .specify/templates/plan-template.md ✅ No changes needed
  - .specify/templates/spec-template.md ✅ No changes needed
  - .specify/templates/tasks-template.md ✅ No changes needed

Follow-up TODOs: None
-->

# llm-mux Constitution

## Core Principles

### I. Code Quality & Safety

All code MUST adhere to Rust safety and quality standards. The workspace enforces:

- `unsafe_code = "forbid"` — no `unsafe` blocks anywhere in the project.
- `clippy::pedantic` and `clippy::nursery` lints as warnings; exceptions require explicit per-file or per-crate
  allow attributes with documented justification.
- `missing_docs = "warn"` — all public items MUST have doc comments.
- `cargo check`, `cargo clippy`, and `cargo fmt` MUST pass clean before merge.
- Error handling MUST use typed enums via `thiserror`; bare `.unwrap()` / `.expect()` in production code paths
  is prohibited — use `Result` propagation or contextual error types.

**Rationale**: A cross-provider protocol translation layer processes untrusted JSON from multiple sources. Memory
safety, exhaustive error handling, and clear documentation are non-negotiable for correctness and maintainability.

### II. Test-First Discipline

Testing is mandatory and MUST precede implementation for new capabilities.

- Cross-protocol roundtrip tests MUST exist for every supported provider ↔ provider combination.
- Each new codec feature (new content type, new protocol field, new stream event) MUST include a test that
  decodes from the source protocol into IR and re-encodes into the target protocol, verifying field-level fidelity.
- Tests reside in `tests/cross_protocol_tests.rs` (integration) and per-crate `tests/` directories.
- Run `cargo test` before every commit; no test regressions are tolerated.
- Where feasible, write the test first, confirm it fails, then implement.

**Rationale**: LLM protocol formats are deeply nested with subtle semantic differences (e.g., OpenAI `finish_reason`
vs. Anthropic `stop_reason`). Without systematic roundtrip coverage, translation errors silently corrupt user
requests and responses.

### III. Cross-Protocol Fidelity

User experience depends on correct, lossless protocol translation. The multiplexer MUST preserve semantic
equivalence across boundaries.

- Every roundtrip (decode → IR → encode) for any supported provider pair MUST be lossless: fields present in
  the source MUST be represented in the target, even if the mapping is not 1:1.
- Unknown fields from the source protocol MUST be carried forward through `ProviderExtensions` or an equivalent
  passthrough mechanism rather than silently dropped.
- The `Converter` trait MUST handle cross-protocol field adaptation; no provider-specific logic in the core IR
  or generic codec code.
- Response encoding MUST be symmetric to request encoding — if a request was translated from Protocol A to
  Protocol B, the response MUST be translated back to Protocol A format before reaching the client.

**Rationale**: Users connect with provider-specific SDKs. A missing field, incorrect stop reason, or lost tool
call ID breaks client integrations and undermines trust in the multiplexer.

### IV. Performance by Design

The multiplexer sits in the critical path of every LLM API call. Performance is a first-class concern.

- Streaming decoding/encoding MUST operate byte-by-byte or event-by-event without buffering the full response
  body in memory.
- Allocations in hot paths (decode/encode per event) MUST be minimized; borrow-based accessors are preferred
  over owned copies for large content (text bodies, base64-encoded images).
- Any new allocation or copy in a decode/encode path that exceeds O(1) per event MUST include a comment
  justifying the cost.
- Backpressure MUST be propagated correctly: if the downstream provider slows, the upstream client MUST
  backpressure, not buffer unboundedly.
- Latency budgets: request translation MUST complete in under 1ms for typical payloads; stream event
  translation MUST complete in under 100µs per event.

**Rationale**: An LLM gateway that adds >1ms overhead per event or buffers entire SSE streams defeats the purpose
of streaming and creates unacceptable latency for real-time chat applications.

### V. Trait-Driven Composability

The architecture is built on Rust traits for extensibility and testability.

- Core abstractions (`Codec`, `Router`, `Converter`, `Authenticator`) are traits in `llm-mux-core`; concrete
  implementations live in separate crates.
- New provider support MUST NOT modify `llm-mux-core` — only add a new codec crate implementing the `Codec` trait.
- The `NoopConverter` is the default; custom `Converter` implementations handle vendor-specific quirks.
- Keep the core IR minimal — add new IR types ONLY when multiple providers share the concept; single-provider
  features use `ProviderExtensions`.
- Follow YAGNI: do not add abstractions for hypothetical future providers. Two providers (OpenAI, Anthropic)
  is sufficient initial scope.

**Rationale**: Trait-based design allows independent development, testing, and benchmarking of each codec,
prevents core churn, and enables third-party codec contributions without touching shared code.

### VI. 代码规模控制 (Code Size Discipline)

保持代码文件和方法的小粒度是长期可维护性的基础。超过合理规模的代码块存在隐含的职责过载、分支过多或
抽象不充分的风险，必须在持续迭代中重构拆分。

- **单个函数/方法**: MUST 不超过 **80 行**（不含注释和空行）。超过时 MUST 拆分为更小的辅助函数，
  每个辅助函数承担单一明确职责。
- **单个源文件**: MUST 不超过 **400 行**（不含注释和空行）。超过时 MUST 将相关逻辑提取为独立模块
  或按职责拆分为多个文件。
- **已有代码的例外**: 在添加新功能时，如果触及超过上述限制的已有代码，MUST 将重构拆分作为功能开发的
  一部分同步完成（"童子军规则"——让代码比你发现时更干净）。
- **不能妥协的场景**: 当函数的核心逻辑必须作为一个整体理解且拆分会导致不合理的参数传递或上下文丢失时，
  MUST 在函数开头添加注释说明不可拆分的理由。此例外不得用于规避重构。
- **合规检查**: `cargo clippy` 结合手动检查。在 CI 中可通过 `tokei` 或类似工具统计行数，超出阈值的
  文件标记为 warning。重构计划阶段（`/speckit.plan`）需要识别待重构的超限文件并纳入任务分解。
- **目录模块组织**: 内部模块（仅被父模块使用、不跨 crate 共享）MUST 优先使用目录结构组织，
  而非平铺的 `_` 前缀文件。例如：`chat/encode.rs`、`chat/stream.rs`（通过目录模块 `chat/mod.rs` 聚合）
  优于 `chat_encode.rs`、`chat_stream.rs`。仅在模块被多个父级使用时，才采用平铺命名。

**Rationale**: 长期项目的历史经验表明，超过 400 行的文件阅读负担呈非线性增长；超过 80 行的函数可测试性
显著下降。协议适配器的 encode/decode 逻辑天然复杂，主动拆分可防止它们演化为不可维护的单体函数。

## Technical Constraints

- **Language**: Rust edition 2021, with `workspace.resolver = "2"`.
- **Serialization**: `serde` + `serde_json` for all protocol parsing and generation.
- **Error handling**: `thiserror` for structured, displayable error types; no `anyhow` in library code.
- **Observability**: `tracing` crate for structured, level-filtered logging; no `println!` or `eprintln!` in
  library code.
- **Dependencies**: All dependencies MUST be declared at workspace level in the root `Cargo.toml` under
  `[workspace.dependencies]`. Crates reference them via `workspace = true`.
- **No unsafe code**: enforced by the workspace lint `unsafe_code = "forbid"`.
- **HTTP server**: `axum` (to be integrated in `llm-mux-gateway`).

## 语言与本地化标准 (Language & Localization)

所有 AI 衍生工件及交互 MUST 统一使用**简体中文（Simplified Chinese）**。此规则覆盖但不限于以下输出：

- **规范与设计文档**: `/speckit.specify` 生成的 `spec.md`、`/speckit.plan` 生成的 `plan.md`、`research.md`、`data-model.md`、`contracts/`、`quickstart.md`
- **任务与清单**: `/speckit.tasks` 生成的 `tasks.md`、`/speckit.checklist` 生成的 `checklists/*.md`
- **日常对话**: 所有 AI 与用户的交互对话
- **代码注释**: 鼓励使用中文注释解释业务逻辑和设计意图（对外公开 API 文档可保留英文）

**例外**:
- 技术栈及架构中已固化的专有名词（如 Kubernetes, Broker, Thread Pool, trait, crate, workspace）可保留英文原文
- 引用的外部英文文档、API 规范、错误消息原文不要求翻译
- 代码标识符（变量名、函数名、类型名）遵循 Rust 惯例使用英文

**合规检查**: 在 Constitution Check 阶段验证所有生成工件的语言一致性；发现非中文工件视为合规违规。

## Development Workflow

- **Code Review**: All changes go through pull requests. At least one approving review is required before merge.
- **CI Gates**: `cargo test`, `cargo clippy`, and `cargo fmt --check` MUST pass on every PR. No exceptions.
- **Documentation**: Public API additions MUST include doc comments with examples. Use `cargo doc` to verify
  documentation builds without warnings.
- **Commit Hygiene**: Commits SHOULD be atomic and well-described. Squash merge is preferred for feature
  branches.
- **Feature Workflow**: Use `/speckit.specify` → `/speckit.clarify` → `/speckit.plan` → `/speckit.tasks` → `/speckit.checklist` → `/speckit.analyze` → `/speckit.implement`
  for structured feature delivery. Constitution compliance is checked at the plan stage.

## Governance

This constitution supersedes all other development practices and conventions. All pull requests and code
reviews MUST verify compliance with the principles herein.

**Amendment Procedure**:
1. Propose changes via pull request to `.specify/memory/constitution.md`.
2. Amendments require discussion and approval from project maintainers.
3. Version number MUST be incremented per semantic versioning:
   - MAJOR: Removal or redefinition of a core principle.
   - MINOR: Addition of a new principle or section.
   - PATCH: Clarifications, wording fixes, non-semantic refinements.
4. Update the Sync Impact Report at the top of this file after every amendment.

**Compliance Review**: Constitution compliance is verified at the `/speckit.plan` stage (Constitution Check
gate) and during code review. Violations MUST be explicitly justified in the plan's Complexity Tracking table
or resolved before merge.

**Guidance File**: `AGENTS.md` provides runtime guidance for AI-assisted development within this project.

**Version**: 1.2.1 | **Ratified**: 2026-05-27 | **Last Amended**: 2026-05-28
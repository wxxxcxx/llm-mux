<!--
Sync Impact Report
==================
Version change: [UNVERSIONED_TEMPLATE] → 1.0.0
Initial population: Filled all placeholder tokens with project-specific values.

Modified principles (all new):
  - PRINCIPLE_1_NAME → I. Code Quality & Safety
  - PRINCIPLE_2_NAME → II. Test-First Discipline
  - PRINCIPLE_3_NAME → III. Cross-Protocol Fidelity
  - PRINCIPLE_4_NAME → IV. Performance by Design
  - PRINCIPLE_5_NAME → V. Trait-Driven Composability

Added sections:
  - Technical Constraints
  - Development Workflow

Removed sections: None

Templates requiring updates:
  - .specify/templates/plan-template.md ✅ No changes needed (Constitution Check gate is generic)
  - .specify/templates/spec-template.md ✅ No changes needed (generic structure)
  - .specify/templates/tasks-template.md ✅ No changes needed (generic structure)
  - .specify/templates/checklist-template.md ✅ No changes needed (generic structure)

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

## Technical Constraints

- **Language**: Rust edition 2021, with `workspace.resolver = "2"`.
- **Serialization**: `serde` + `serde_json` for all protocol parsing and generation.
- **Error handling**: `thiserror` for structured, displayable error types; no `anyhow` in library code.
- **Observability**: `tracing` crate for structured, level-filtered logging; no `println!` or `eprintln!` in
  library code.
- **Dependencies**: All dependencies MUST be declared at workspace level in the root `Cargo.toml` under
  `[workspace.dependencies]`. Crates reference them via `workspace = true`.
- **No unsafe code**: enforced by the workspace lint `unsafe_code = "forbid"`.
- **HTTP server**: `axum` (to be integrated in `llm-mux-server`).

## Development Workflow

- **Code Review**: All changes go through pull requests. At least one approving review is required before merge.
- **CI Gates**: `cargo test`, `cargo clippy`, and `cargo fmt --check` MUST pass on every PR. No exceptions.
- **Documentation**: Public API additions MUST include doc comments with examples. Use `cargo doc` to verify
  documentation builds without warnings.
- **Commit Hygiene**: Commits SHOULD be atomic and well-described. Squash merge is preferred for feature
  branches.
- **Feature Workflow**: Use `/speckit.specify` → `/speckit.plan` → `/speckit.tasks` → `/speckit.implement`
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

**Version**: 1.0.0 | **Ratified**: 2026-05-27 | **Last Amended**: 2026-05-27


## 语言与本地化标准 (Language & Localization)
- **Lingua Franca**: 本项目的所有 AI 衍生工件（包括不限于 `/` 生成的 `spec.md`、`/plan` 生成的 `plan.md`、`/tasks` 生成的任务列表以及日常对话）**必须统一使用简体中文（Simplified Chinese）**进行输出。
- **专有名词约束**: 技术栈及架构中的专有名词（如 Kubernetes, Broker, Thread Pool）可保留英文，但整体语法、描述和验收标准必须为中文。
# Implementation Plan: Model Config Parity

**Branch**: `008-model-config-parity` | **Date**: 2026-11-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-model-config-parity/spec.md`

## Summary

Leverage the `model2vec-rs` 0.5 API (merged via PR #171) to (a) resolve each
model's maximum input length from its own `config.json` with an explicit
operator override, (b) accept a per-request `normalize` boolean on the
TEI-compatible embed endpoints, and (c) report each model's true
normalization setting and effective input limit in `/info`. No new
dependencies; all changes are within the existing axum/clap/model2vec-rs
stack.

## Technical Context

**Language/Version**: Rust 2024 edition; MSRV tracks the stable toolchain used by CI and the Docker builder (currently 1.98).

**Primary Dependencies**: `axum`, `tokio`, `model2vec-rs` 0.5 (new API: `max_length()`, `normalize()`, per-call normalize parameter on `encode_with_args`), `clap`, `serde`, `tracing`, `utoipa`, `thiserror`, `anyhow`. No new dependencies.

**Storage**: N/A — all state is in memory; models are loaded at startup and shared via `Arc<AppState>`.

**Testing**: `cargo test` (unit, contract `tests/*_contract.rs`, integration `tests/*_integration.rs`, `tests/config_unit.rs`), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt -- --check`, `cargo bench`.

**Target Platform**: Linux container and Kubernetes via the Helm chart (it-at-m/helm-charts); local development on Linux.

**Project Type**: web-service / HTTP API.

**Performance Goals**: unchanged from existing goals (p99 < 20 ms batch-1, ≥ 2,000 RPS per model); per-call normalization adds no measurable overhead (it is an existing code path parameter).

**Constraints**: `unsafe_code = "forbid"`; `unwrap_used = "deny"`; clippy pedantic warnings are errors in CI; `missing_docs = "warn"`; OpenAI contract shapes MUST NOT change; additive TEI/`/info` changes only (minor version bump, no MAJOR).

**Scale/Scope**: Two TEI embed handlers, TEI info handlers, config resolution, model metadata plumbing; one optional integration test model with a declared `max_length`.

**Research Resolution**: All unknowns resolved in [research.md](./research.md):
1. `max_length: null` in `config.json` falls back to `512` (never unbounded).
2. Precedence is explicit operator override → model config → crate default `512`.
3. OpenAI endpoints gain no normalization parameter (contract rule III).
4. TEI-style `truncate` parameter is deferred (out of scope).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | Pass | Resolution logic extracted into a small, testable function; no `unwrap`/`expect`; new public items documented (`missing_docs = "warn"`). |
| II. Test Coverage | Pass with action | New config resolution unit tests (`tests/config_unit.rs`); contract tests for `normalize` on both TEI embed endpoints and updated `/info` shape; integration test with a model declaring `max_length`. Tests defined before implementation. |
| III. API Conformity | Pass with action | OpenAI contract unchanged (FR-5). TEI and `/info` changes are additive and documented; OpenAPI (`/docs`) kept in sync via utoipa annotations. No breaking change → minor version bump. |
| IV. Simplicity Over Complexity | Pass | Reuses the existing `--max-input-length` flag (`Option<usize>` sentinel) instead of new flags or a per-model config table. No new dependencies. |
| V. Performance Focus | Pass | No additional per-request work: resolution happens at model load time; `normalize` maps directly onto the existing `encode_with_args` parameter. Benchmarks run before release; ≤ 10% regression budget. |

### Post-Design Re-Check

After completing Phase 1 design artifacts (`data-model.md`, `contracts/`, `quickstart.md`):

- **I. Code Quality**: still Pass — precedence helper is a pure function; handlers stay thin.
- **II. Test Coverage**: still Pass with action — contracts cover `normalize` default (absent) semantics, explicit `true`/`false`, and `/info` fields; edge cases (invalid `max_length` type) covered by the existing failed-model path plus a unit test on resolution.
- **III. API Conformity**: still Pass with action — additive fields documented; release notes must call out the behavior change for models that declare `max_length` (SC-2 guards the default case).

## Project Structure

### Documentation (this feature)

```text
specs/008-model-config-parity/
├── plan.md              # This file (/speckit.plan command output)
├── spec.md              # Feature specification (drafted with this plan)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── config.rs                # max_input_length → Option<usize>, resolution helper
├── state.rs                 # wires resolved config into registry loading
├── model/
│   ├── mod.rs               # LoadedModel: resolved per-model max_input_length
│   └── embedding.rs         # max_length()/normalize() accessors, normalize param
└── routes/
    ├── dto.rs               # TeiEmbedRequest.normalize, ModelInfo.normalize
    ├── tei.rs               # pass normalize through; populate ModelInfo
    └── embeddings.rs        # unchanged (OpenAI contract)

tests/
├── config_unit.rs           # resolution precedence table
├── tei_contract.rs          # normalize field on both embed endpoints
├── info_contract.rs         # /info normalize + per-model max_input_length
└── *_integration.rs         # per-model truncation with declared max_length

docs/                        # VitePress: TEI page, config reference, /info response
```

**Structure Decision**: Single-project web service (existing layout). All changes fit the established module boundaries: config in `src/config.rs`, model metadata in `src/model/`, DTOs and handlers in `src/routes/`, tests in `tests/`. No new top-level modules.

## Complexity Tracking

> No constitution violations; this section is intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| — | — | — |

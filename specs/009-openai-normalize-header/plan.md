# Implementation Plan: OpenAI Normalize Header

**Branch**: `009-openai-normalize-header` | **Date**: 2026-11-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-openai-normalize-header/spec.md`

## Summary

Extend `POST /v1/embeddings` with an optional `X-Normalize` request header
that gives OpenAI-compatible clients per-request control over L2
normalization — `true` forces unit-length vectors, `false` returns raw
mean-pooled vectors, and absence keeps the model's configured default — while
leaving the OpenAI request and response JSON contract byte-identical. Invalid
header values return the standard `400` error body. Builds on the per-request
normalization plumbing from feature 008 (PR #174); no new dependencies.

## Technical Context

**Language/Version**: Rust 2024 edition; MSRV tracks the stable toolchain used by CI and the Docker builder (currently 1.98).

**Primary Dependencies**: `axum` (header extraction via `HeaderMap`), `model2vec-rs` 0.5, `utoipa` (header parameter documentation), existing `errors.rs` `AppError::BadRequest`. No new dependencies.

**Storage**: N/A — the header is stateless per-request input; no persistence.

**Testing**: `cargo test` (contract tests in `tests/openai_contract.rs`, integration in `tests/openai_integration.rs`, regression via the full suite), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt -- --check`.

**Target Platform**: Linux container and Kubernetes via the Helm chart (it-at-m/helm-charts); local development on Linux.

**Project Type**: web-service / HTTP API.

**Performance Goals**: unchanged (p99 < 20 ms batch-1); header parsing is O(1) string comparison per request and adds no measurable overhead.

**Constraints**: `unsafe_code = "forbid"`; `unwrap_used = "deny"`; clippy pedantic warnings are errors; OpenAI request/response JSON shapes MUST remain byte-identical for all header states (spec FR-005); the extension MUST be documented as an explicit service deviation (Constitution III).

**Scale/Scope**: one endpoint (`/v1/embeddings`), one new pure parsing function, handler wiring, contract/integration tests, docs updates. TEI endpoints explicitly out of scope.

**Research Resolution**: All unknowns resolved in [research.md](./research.md):
1. Canonical header name is `X-Normalize` (HTTP header names are case-insensitive on the wire).
2. Value grammar: case-insensitive `true` / `false` only; everything else — including `1`/`0` — is rejected with `400 invalid_request`.
3. Validation order: body validation errors take precedence over header errors (spec Edge Cases), with malformed-JSON extraction errors naturally preceding both.
4. Duplicate headers are rejected with `400` rather than "first value wins".

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | Pass | Header parsing extracted into a small pure function with explicit error cases; no `unwrap`/`expect`; documented public items. |
| II. Test Coverage | Pass with action | Contract tests for all four header states (absent/true/false/invalid) plus duplicate-header and case-insensitivity cases; integration tests verify norm differences and unchanged response shape; tests written first. |
| III. API Conformity | Pass with action | JSON request/response contract unchanged (FR-005). The header is a *documented* service extension: covered by FR-007/FR-008, an API-contract doc under `contracts/`, release notes, and OpenAPI annotations in `/docs`. Additive → minor version bump. |
| IV. Simplicity Over Complexity | Pass | A single optional header reuses the existing `EmbeddingModel::encode` normalize parameter from 008 — no new flags, config, DTO fields, or dependencies. |
| V. Performance Focus | Pass | Constant-time header parse; same encode path as 008. Benchmarks unchanged; ≤ 10% regression budget retained. |

### Post-Design Re-Check

After completing Phase 1 design artifacts (`data-model.md`, `contracts/`, `quickstart.md`):

- **I. Code Quality**: still Pass — parsing function is pure and unit-testable; handler stays thin.
- **II. Test Coverage**: still Pass with action — contract tests enumerate every acceptance scenario from the spec (absent default, forced on/off, invalid/empty/duplicate header, case-insensitivity, base64 encoding format).
- **III. API Conformity**: still Pass with action — `contracts/openai_embeddings.md` documents the extension and marks it as a deviation exception; release notes required at merge time.

## Project Structure

### Documentation (this feature)

```text
specs/009-openai-normalize-header/
├── plan.md              # This file (/speckit.plan command output)
├── spec.md              # Feature specification
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── routes/
│   ├── embeddings.rs    # Header extraction, parse, pass-through to encode
│   ├── dto.rs           # unchanged (JSON contract untouched)
│   └── mod.rs           # unchanged routing
├── config.rs            # unchanged
├── errors.rs            # unchanged (reuse AppError::BadRequest / invalid_request)
└── model/embedding.rs   # unchanged (normalize parameter exists since 008)

tests/
├── openai_contract.rs   # header state matrix + error cases
├── openai_integration.rs# norm differences, response shape, base64
docs/
└── api/openai.md        # header documentation + extension note
```

**Structure Decision**: Single-project web service (existing layout). All changes concentrate in `src/routes/embeddings.rs` plus tests and docs; the parsing helper lives next to its only consumer. No new modules.

## Complexity Tracking

> No constitution violations; this section is intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| — | — | — |

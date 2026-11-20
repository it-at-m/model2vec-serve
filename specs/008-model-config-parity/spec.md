# Feature Specification: Model Config Parity

**Feature Branch**: `008-model-config-parity`

**Created**: 2026-11-20

**Status**: Draft

**Input**: User description: "Leverage model2vec-rs 0.5: derive each model's
maximum input length from its own config.json instead of a single global CLI
value, expose a per-request `normalize` flag on TEI embed endpoints, and report
the model's true normalization setting in `/info`." (From the review of Renovate
PR #171.)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Per-Model Maximum Input Length (Priority: P1)

As an operator serving multiple models from one process, I want each model to
truncate inputs at its own configured `max_length` (from the model's
`config.json`) so that a 256-token code model and a 512-token multilingual
model each behave exactly as their authors intended, without me running
separate deployments or choosing one global compromise.

**Why this priority**: Truncation length directly changes embedding output.
A wrong global limit silently degrades every embedding of every model except
the one it was tuned for; this is the highest-impact correction enabled by
`model2vec-rs` 0.5.

**Independent Test**: Load two models whose `config.json` declares different
`max_length` values, send inputs longer than the smaller limit, and verify
each model truncates at its own limit while `/info` reports the per-model
value.

**Acceptance Scenarios**:

1. **Given** a model whose `config.json` declares `max_length: 256` and no explicit operator override, **When** the service starts and `/info` is queried, **Then** `max_input_length` is `256` for that model.
2. **Given** a model whose `config.json` omits `max_length`, **When** the service starts, **Then** the effective limit falls back to `512` (the crate default) and `/info` reports `512`.
3. **Given** the operator sets `--max-input-length` / `MAX_INPUT_LENGTH` explicitly, **When** models load, **Then** the explicit value overrides every model's configured value and a startup log records the resolution.
4. **Given** a model whose `config.json` declares `max_length: null`, **When** the service starts, **Then** the effective limit falls back to `512` and the fallback is logged (the service never runs truly unlimited truncation).

### User Story 2 - Per-Request Normalization on TEI Endpoints (Priority: P2)

As a TEI-compatible client, I want to pass an optional `normalize` boolean in
the embed request body so that I can obtain raw (unnormalized) or L2-normalized
embeddings per request without redeploying the service.

**Why this priority**: Purely additive client-facing capability; enables
parity with TEI-style clients that expect normalization control. Lower
priority because current behavior (model default) remains correct for all
existing clients.

**Independent Test**: Send two identical embed requests, one with
`"normalize": false` and one with `"normalize": true`, and verify the vectors
differ in norm accordingly.

**Acceptance Scenarios**:

1. **Given** a TEI embed request without `normalize`, **When** embeddings are produced, **Then** behavior is identical to the current service (model config default).
2. **Given** a TEI embed request with `"normalize": false`, **When** embeddings are produced, **Then** vectors are returned without L2 normalization regardless of the model default.
3. **Given** a TEI embed request with `"normalize": true`, **When** embeddings are produced, **Then** vectors are L2-normalized regardless of the model default.
4. **Given** the OpenAI-compatible `/v1/embeddings` endpoint, **When** a request is sent, **Then** no normalization parameter exists or is honored (the OpenAI contract has no such field).

### User Story 3 - Accurate Model Metadata in `/info` (Priority: P3)

As a client or operator inspecting the service, I want `/info` to report each
model's actual normalization setting and its effective maximum input length so
that I can validate client assumptions against the deployed models instead of
guessing.

**Why this priority**: Metadata accuracy; no behavior change, but it makes
User Stories 1 and 2 verifiable by clients.

**Independent Test**: Query `/info` (root and per-model) and compare the
reported `normalize` and `max_input_length` values against the model's
`config.json` and the resolved configuration.

**Acceptance Scenarios**:

1. **Given** any loaded model, **When** `/info` is queried, **Then** the response contains a `normalize` boolean matching the model's configured normalization.
2. **Given** models with different effective limits, **When** per-model `/info` is queried, **Then** each model reports its own `max_input_length`.

### Edge Cases

- A model's `config.json` contains an invalid `max_length` type (e.g. a string): the model fails to load with a clear startup error (upstream crate behavior), and the service reports the failure through the existing failed-model status path.
- A request body with `"normalize": null` is treated as absent (serde default).
- All configured models fail to load: existing startup error behavior is unchanged.
- A model declares `max_length` larger than a globally configured `MAX_INPUT_LENGTH`: the explicit operator value wins and a warning is logged at startup.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-1**: The service MUST resolve each model's maximum input length per model, with the precedence: (1) explicit operator `--max-input-length` / `MAX_INPUT_LENGTH`, (2) the model's `config.json` `max_length`, (3) fallback `512`.
- **FR-2**: A `max_length` of `null` in a model's `config.json` MUST fall back to `512`; the service MUST NOT run with unbounded truncation.
- **FR-3**: `LoadedModel.max_input_length` MUST hold the resolved per-model value, and `/info` (root and per-model) MUST report it.
- **FR-4**: The TEI embed endpoints (`/embed` and `/tei/{model_id}/embed`) MUST accept an optional `normalize: boolean` field; absent/null means the model's configured default.
- **FR-5**: The OpenAI-compatible endpoints MUST NOT gain a normalization parameter.
- **FR-6**: `/info` responses MUST include a `normalize` boolean reflecting the model's configured normalization behavior.
- **FR-7**: The service MUST log each model's resolved `max_input_length` at startup, and warn when an explicit override replaces a model's configured value.
- **FR-8**: OpenAPI documentation (`/docs`) MUST reflect the `normalize` field and the updated `ModelInfo` schema.

### Key Entities *(include if feature involves data)*

- **Config**: `max_input_length` becomes `Option<usize>` — `None` means "not explicitly set, resolve per model".
- **LoadedModel**: holds the resolved per-model `max_input_length`; unchanged externally.
- **EmbeddingModel**: exposes the loaded model's `max_length` and `normalize` accessors; encode methods accept a per-call normalization override.
- **TeiEmbedRequest**: gains optional `normalize: Option<bool>`.
- **ModelInfo**: gains `normalize: bool`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-1**: For models declaring `max_length` in `config.json`, `/info` reports the declared value when no operator override is set (100% of validation models).
- **SC-2**: Existing client requests (no `normalize` field, no `MAX_INPUT_LENGTH` set, models without configured `max_length`) produce byte-identical embeddings to the pre-change service.
- **SC-3**: `normalize: false` and `normalize: true` requests produce vectors whose L2 norms differ as expected (≈1.0 vs. unnormalized).
- **SC-4**: All contract and integration tests pass; benchmarks show no regression greater than 10%.

## Assumptions

- `model2vec-rs` 0.5 is merged (PR #171) before implementation starts.
- The crate default of `512` is an acceptable universal fallback, matching the service's historical default.
- Model authors' `config.json` values are authoritative when the operator has not overridden them.

# Research: Model Config Parity

## Decision: Handling of `max_length: null` in a model's config.json

- **Decision**: Treat `null` as "the model declares no limit" and fall back to the service default `512`. The service never runs with unbounded truncation.
- **Rationale**: Unbounded truncation would let request size drive tokenization cost and memory; the crate's `null` semantics (`max_length: Option<usize>` = no truncation) are safe for a library but not for a multi-tenant HTTP service. Falling back to `512` matches the service's historical default and the crate's own default when `max_length` is absent (`DEFAULT_MAX_LENGTH = 512`), so `null` and absent behave identically. The model's effective value is logged at startup so operators can see the fallback.
- **Alternatives considered**:
  - Honoring `null` as truly unlimited — rejected: unbounded per-request work conflicts with the service's timeout and batch-size safeguards and Constitution principle V.
  - Rejecting models with `max_length: null` at load — rejected: needlessly strict; no current model2vec model declares `null`.

## Decision: Resolution precedence for the maximum input length

- **Decision**: Per model: (1) explicit operator `--max-input-length` / `MAX_INPUT_LENGTH` wins; (2) otherwise the model's `config.json` `max_length`; (3) otherwise `512`.
- **Rationale**: An explicit operator override is a deliberate act and must remain authoritative — this preserves the pre-0.5 deployment story (chart values, compose env) unchanged. Making the CLI value `Option<usize>` distinguishes "explicitly set" from "defaulted", which is exactly the information needed to decide between (1) and (2) without adding a new flag (Constitution principle IV).
- **Alternatives considered**:
  - A separate `--per-model-max-length` toggle flag — rejected: two overlapping flags are more complex than one `Option<usize>` argument.
  - Always preferring the model config and ignoring the CLI value — rejected: breaks existing deployments that rely on the override and removes operator control.
  - Per-model CLI entries (`--max-input-length model=256`) — rejected: no demonstrated need; YAML/env per-model configuration would be a larger surface better handled in a future feature if operators ask for it.

## Decision: No normalization parameter on OpenAI-compatible endpoints

- **Decision**: `/v1/embeddings` keeps its exact contract (`input`/`model`/`encoding_format`); no `normalize` field.
- **Rationale**: Constitution principle III (API Conformity): the OpenAI embeddings schema has no normalization parameter, and adding one would create a silent deviation from the documented OpenAI contract. Clients needing normalization control use the TEI endpoints (FR-4), where such a field is consistent with TEI-style client expectations and is additive, not breaking.
- **Alternatives considered**:
  - Accepting and ignoring a `normalize` field on `/v1/embeddings` — rejected: accepting unknown fields silently is worse than rejecting; OpenAI-compatible clients never send it, so strictness costs nothing.
  - A query parameter on `/v1/embeddings` — rejected: query parameters for body-level behavior are non-idiomatic and would still be a documented deviation.

## Decision: Deferral of the TEI-style `truncate` parameter

- **Decision**: Out of scope for this feature; the service always truncates to the resolved maximum input length.
- **Rationale**: `model2vec-rs` truncation is always-on and cheap; a client-side truncate toggle adds a second orthogonal knob whose interaction with per-model limits would need its own contract work. Deferring keeps this feature minimal.
- **Alternatives considered**: Implementing `truncate: bool` alongside `normalize` — rejected for scope; revisit if TEI clients report a need.

## Decision: Where the resolved limit lives

- **Decision**: Resolve once at model load time and store the result in `LoadedModel.max_input_length` (type stays `usize`).
- **Rationale**: Handlers already read `loaded.max_input_length`; keeping the type and call sites unchanged minimizes the diff and avoids re-resolving per request. Resolution is a pure function `resolve_max_input_length(explicit: Option<usize>, model_max: Option<usize>) -> usize`, unit-testable in `tests/config_unit.rs`.
- **Alternatives considered**:
  - Storing `Option<usize>` and resolving in handlers — rejected: pushes branching into five call sites and makes `/info` reporting error-prone.
  - Re-reading `config.json` per request — rejected: pointless I/O and inconsistent view of state.

## Decision: Startup observability of the resolution

- **Decision**: Log each model's resolved limit at startup (`tracing::info!`), and emit a `tracing::warn!` when an explicit override replaces a model's configured value.
- **Rationale**: The behavior change for models that declare `max_length` is the only non-obvious operational consequence of this feature; a warning makes misconfiguration (override fighting the model author's intent) visible without failing startup.
- **Alternatives considered**: Failing startup on conflict — rejected: the override is legitimate operator intent, not an error.

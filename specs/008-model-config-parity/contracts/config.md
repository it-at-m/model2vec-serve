# Contract: Configuration — Per-Model Maximum Input Length

Applies to the `--max-input-length` CLI argument and the `MAX_INPUT_LENGTH`
environment variable after feature `008-model-config-parity`. The flag and env
names are unchanged; only the default and resolution semantics change.

## CLI / Environment

| Item | Value |
|------|-------|
| Argument | `--max-input-length` |
| Environment | `MAX_INPUT_LENGTH` |
| Type | positive integer |
| Default | **unset** (previously `512`) |

## Resolution (per model, evaluated once at model load)

| Explicit `MAX_INPUT_LENGTH` | Model `config.json` `max_length` | Effective limit | `/info` reports |
|---|---|---|---|
| set | any | the explicit value | the explicit value |
| unset | `m` (integer) | `m` | `m` |
| unset | absent | `512` (crate default) | `512` |
| unset | `null` | `512` (fallback; never unbounded) | `512` |

## Startup Logging

- One `info` line per loaded model records the resolved limit.
- When an explicit override replaces a model's configured value, a `warn`
  line records the model id, the override, and the model's configured value.
- A model whose `config.json` contains an invalid `max_length` type fails to
  load (upstream crate behavior) and is reported through the existing
  failed-model status path on `/health`; the service still starts if at least
  one model loads.

## Backward Compatibility

- Deployments that explicitly set `MAX_INPUT_LENGTH` behave identically to
  before (the value applies to all models).
- Deployments that do not set it and serve models without `max_length` in
  their `config.json` behave identically to before (`512`).
- Deployments serving models that declare `max_length` change truncation
  behavior intentionally; this is called out in the release notes.

## Non-Goals

- No per-model CLI entries (e.g. `--max-input-length model=256`); a single
  optional global override remains the full operator surface.
- No changes to other configuration fields.

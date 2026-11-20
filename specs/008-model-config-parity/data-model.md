# Data Model: Model Config Parity

## Entities

### Config (src/config.rs)

Runtime configuration for the service; the only field that changes type.

| Field | Type | Required | Change | Description |
|-------|------|----------|--------|-------------|
| max_input_length | `Option<usize>` | no | **changed** from `usize` (default 512) to `Option<usize>` | `Some(n)` = explicit operator override applied to all models; `None` = resolve per model from each model's `config.json`, falling back to `512`. CLI `--max-input-length` / env `MAX_INPUT_LENGTH` unchanged in spelling; the default is now "unset". |

All other fields are unchanged.

**Validation rules**: `Some(0)` is rejected at startup (a zero token limit cannot produce embeddings); values are positive otherwise. Reuse of the existing positive-integer validation pattern applies.

### LoadedModel (src/model/mod.rs)

Metadata and a loaded model2vec instance for a single model.

| Field | Type | Description | Change |
|-------|------|-------------|--------|
| model_id | `String` | Canonical identifier used for routing and responses. | unchanged |
| max_input_length | `usize` | **Resolved** effective maximum tokens per input for this model. | semantics changed: per-model resolution at load time, replaces global value |
| embedding_dimension | `usize` | Size of each embedding vector. | unchanged |
| pooling | `&'static str` | Pooling method, `"mean"`. | unchanged |
| model | `Arc<EmbeddingModel>` | The underlying model instance. | unchanged (inner type gains accessors) |

### EmbeddingModel (src/model/embedding.rs)

Wrapper around a loaded `StaticModel`.

| Member | Signature | Description |
|--------|-----------|-------------|
| max_length | `pub fn max_length(&self) -> Option<usize>` | The model's configured `max_length` from `config.json` (`None` = declared no limit). **new** |
| normalize | `pub fn normalize(&self) -> bool` | The model's configured L2-normalization setting. **new** |
| encode | `pub fn encode(&self, inputs: &[String], max_length: usize, batch_size: usize, normalize: Option<bool>) -> Vec<Vec<f32>>` | Batch encode; `normalize` overrides the model default per call. **signature changed** |
| encode_single | `pub fn encode_single(&self, input: &str, max_length: usize, normalize: Option<bool>) -> Vec<f32>` | Single-input encode with the same override. **signature changed** |

### Resolution Function (pure, unit-tested)

```text
resolve_max_input_length(explicit: Option<usize>, model_max: Option<usize>) -> usize
```

| explicit (CLI/env) | model config `max_length` | result |
|--------------------|---------------------------|--------|
| `Some(n)` | any (including `null`/absent) | `n` (override wins; warn if model declared a different value) |
| `None` | `Some(m)` | `m` |
| `None` | absent or `null` | `512` (crate default; never unbounded) |

### TeiEmbedRequest (src/routes/dto.rs)

| Field | Type | Required | Change | Description |
|-------|------|----------|--------|-------------|
| inputs | `TeiInput` | yes | unchanged | Single string, string array, or pre-tokenized input. |
| normalize | `Option<bool>` | no | **new** | `None`/absent/`null` = model's configured default; `true` = L2-normalize; `false` = raw sums. Applies to `/embed` and `/tei/{model_id}/embed`. |

### ModelInfo (src/routes/dto.rs)

| Field | Type | Required | Change | Description |
|-------|------|----------|--------|-------------|
| model_id | `String` | yes | unchanged | Model identifier. |
| max_input_length | `usize` | yes | semantics changed | Now the resolved per-model value (see LoadedModel). |
| embedding_dimension | `usize` | yes | unchanged | — |
| pooling | `String` | yes | unchanged | `"mean"`. |
| normalize | `bool` | yes | **new** | The model's configured normalization setting, from `StaticModel::normalize()`. |

## State Transitions

Not applicable — entities are immutable after startup (models load once; the
registry is read-only at request time). The only stateful moment is model
loading, where the resolution function runs and the result is logged.

## Relationships

- `Config.max_input_length` is consumed once by `ModelRegistry::load`, which
  passes the explicit value (or `None`) into `LoadedModel::load` alongside the
  model path; `LoadedModel` stores the resolved result.
- `TeiEmbedRequest.normalize` flows: handler → `EmbeddingModel::encode` →
  `StaticModel::encode_with_args(..., normalize)`. No state is stored.
- `ModelInfo.normalize` and `ModelInfo.max_input_length` are derived from
  `LoadedModel` when `/info` handlers build their response.

# Data Model: OpenAI Normalize Header

## Entities

### Embedding Request Headers

An optional client-supplied header attached to an OpenAI embeddings request.
Stateless: consumed once per request, never persisted, never echoed in
responses.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| X-Normalize | `Option<bool>` (parsed) | no | Header value after parsing. Absent → `None` (model's configured default). `true` → force L2 normalization. `false` → raw mean-pooled vectors. Any other value → request rejected before inference. |

**Value states**:

| Raw header value | Parsed | Behavior |
|------------------|--------|----------|
| (absent) | `None` | model's configured normalization default (pre-change behavior) |
| `true` (any case) | `Some(true)` | embeddings L2-normalized |
| `false` (any case) | `Some(false)` | raw mean-pooled embeddings |
| anything else, empty, or duplicated | — | `400 invalid_request`, standard error body |

**Validation rules** (from spec FR-004 and research decisions):

- Exactly zero or one `X-Normalize` value per request; more than one → `400`.
- Value must equal `true` or `false` case-insensitively; HTTP does not trim header values, so recognized spellings are the exact strings `true` and `false` in any letter case (with no surrounding whitespace).
- Validation runs **after** body validation: a request invalid in both respects surfaces the body error.

### Embedding Response (unchanged)

| Field | Type | Change |
|-------|------|--------|
| object | `&'static str` ("list") | unchanged |
| data | array of `{object, index, embedding}` | unchanged shape; vector values reflect the requested normalization |
| model | `String` | unchanged |
| usage | `{prompt_tokens, total_tokens}` | unchanged |

The header never appears in the response (FR-005).

## State Transitions

Not applicable — the header is per-request input with no lifecycle.

## Relationships

- The parsed `Option<bool>` flows: header → embeddings handler →
  `EmbeddingModel::encode(..., normalize)` (the same parameter the TEI body
  field feeds since feature 008). No state is stored; metrics labeling
  (method/path/status/model) is unchanged by the header's value.

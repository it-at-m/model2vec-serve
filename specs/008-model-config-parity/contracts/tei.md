# Contract: TEI Endpoints — `normalize` and Per-Model Metadata

Applies to `/embed`, `/tei/{model_id}/embed`, `/info`, and
`/tei/{model_id}/info` after feature `008-model-config-parity`. All changes
are additive; requests that omit the new field behave exactly as before.

## `POST /embed` and `POST /tei/{model_id}/embed`

**Headers** (unchanged):
- `Content-Type: application/json`
- `Authorization: Bearer <api_key>` (when authentication is enabled)

**Body** (new field):

```json
{
  "inputs": ["Hello world", "Second text"],
  "normalize": false
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| inputs | string \| string[] \| token arrays | yes | — | Unchanged; same accepted shapes as today. |
| normalize | boolean | no | model's configured default | `true` = L2-normalize the pooled sums; `false` = return the raw mean sums; `null`/absent = model's configured default. |

**Response** (unchanged shape):

**Status**: `200 OK`

```json
[
  [0.0123, -0.0456, ...],
  [0.0321, -0.0654, ...]
]
```

Vectors are returned in input order. With `normalize: true` each vector has
L2 norm ≈ 1.0; with `normalize: false` norms reflect the raw pooled sums.

## `GET /info` and `GET /tei/{model_id}/info`

**Response** (one new field):

**Status**: `200 OK`

```json
{
  "model_id": "minishlab/blink-multilingual-64M",
  "max_input_length": 512,
  "embedding_dimension": 384,
  "pooling": "mean",
  "normalize": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| model_id | string | Unchanged. |
| max_input_length | integer | **Changed semantics**: the model's resolved effective limit — explicit `MAX_INPUT_LENGTH` override, else the model's `config.json` `max_length`, else `512`. |
| embedding_dimension | integer | Unchanged. |
| pooling | string | Unchanged (`"mean"`). |
| normalize | boolean | **New**: the model's configured L2-normalization setting. |

## Errors

No new error codes. `normalize` accepts any boolean; `null` is treated as
absent. Invalid `inputs` continues to return `400 Bad Request` with the
standard error body (`{"error": "code", "message": "..."}`) as documented in
the existing error contracts.

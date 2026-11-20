# Contract: OpenAI-compatible Embeddings — `X-Normalize` Extension

Extends the base OpenAI embeddings contract
(`specs/001-model2vec-embedding-api/contracts/openai_embeddings.md`) with an
optional request header. **The request body and response body are unchanged**
— this is the only extension, documented per the project's API conformity
governance as an explicit, user-visible service deviation.

## Endpoint

`POST /v1/embeddings`

## Request

**Headers** (extension in bold):

- `Content-Type: application/json`
- `Authorization: Bearer <api_key>` (when authentication is enabled)
- **`X-Normalize` (optional, service-specific extension)**

**`X-Normalize` values**:

| Value | Behavior |
|-------|----------|
| (absent) | Model's configured normalization default — byte-identical to the pre-extension service |
| `true` (case-insensitive) | Embeddings are L2-normalized; each vector has norm ≈ 1.0 |
| `false` (case-insensitive) | Embeddings are the raw mean-pooled vectors (norm reflects pooling output) |
| anything else | `400 invalid_request`, standard error body |
| duplicate header (two or more values) | `400 invalid_request`, standard error body |

**Body** (unchanged):

```json
{
  "input": "Hello world",
  "model": "minishlab/blink-multilingual-64M",
  "encoding_format": "float"
}
```

## Response

**Status**: `200 OK` — body shape identical to the base contract for every
header state; the header is never echoed. Vector values reflect the requested
normalization (including with `encoding_format: "base64"`).

## Errors

| Status | `error` code | Cause |
|--------|--------------|-------|
| `400` | `invalid_request` | Unrecognized or empty `X-Normalize` value, duplicate header, or any pre-existing body validation failure |

**Precedence**: when a request is invalid in more than one respect, existing
body validation errors are reported first; the header error surfaces only for
requests whose body is valid. Malformed JSON (rejected at body extraction)
also takes precedence.

## Scope

- The header is read **only** by `POST /v1/embeddings`.
- TEI endpoints (`/embed`, `/tei/{model_id}/embed`) ignore it; their
  normalization mechanism remains the optional `normalize` body field
  (feature 008, `specs/008-model-config-parity/contracts/tei.md`).
- Unknown headers on other endpoints remain ignored, as before.

## Governance

This extension is an approved deviation from the OpenAI contract in its
mildest form: no JSON schema change, additive, and documented on the OpenAI
API docs page, the interactive OpenAPI docs (`/docs`), this contract, and the
release notes.

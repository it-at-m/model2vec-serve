# Quickstart: OpenAI Normalize Header

This guide validates feature `009-openai-normalize-header` end-to-end after
implementation. Contract details: [contracts/openai_embeddings.md](./contracts/openai_embeddings.md);
entity definitions: [data-model.md](./data-model.md).

## Prerequisites

- Rust toolchain (stable, matching CI; currently 1.98)
- Feature 008-model-config-parity merged (per-request normalization plumbing)
- `curl`, `jq`, `python3` (for norm calculation)

## 1. Absent header keeps existing behavior (FR-002, SC-001)

```bash
curl -s http://localhost:8080/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{"input": ["hello"], "model": "minishlab/blink-multilingual-64M"}' | jq '.data[0].embedding | length'
```

Expected: standard OpenAI response shape, unchanged from before the feature.
The response body must contain no reference to the header.

## 2. Header "on" forces unit norm (FR-003, SC-002)

```bash
curl -s http://localhost:8080/v1/embeddings \
  -H 'Content-Type: application/json' \
  -H 'X-Normalize: true' \
  -d '{"input": ["hello"], "model": "minishlab/blink-multilingual-64M"}' \
  | jq '.data[0].embedding | map(. * .) | add | sqrt'
```

Expected: norm ≈ 1.0 (tolerance ~1e-2), regardless of the model default.

## 3. Header "off" returns raw pooled vectors (FR-003, SC-002)

Same request with `-H 'X-Normalize: false'`.

Expected: norm differs from the `true` case (for a normalized model) while
the response shape stays identical. Repeating with mixed casing
(`x-normalize: FALSE`) must behave the same.

## 4. Invalid values fail with the standard error (FR-004, SC-003)

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8080/v1/embeddings \
  -H 'Content-Type: application/json' \
  -H 'X-Normalize: maybe' \
  -d '{"input": ["hello"]}'
```

Expected: `400` and the standard error body `{"error": "invalid_request",
"message": "..."}`. Repeat with an empty value and a duplicated header
(`-H 'X-Normalize: true' -H 'X-Normalize: false'`) — same outcome.

## 5. Body errors take precedence (Edge Cases)

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8080/v1/embeddings \
  -H 'Content-Type: application/json' \
  -H 'X-Normalize: maybe' \
  -d '{"input": []}'
```

Expected: `400` with the *body* error message (empty input), not the header
error.

## 6. TEI endpoints ignore the header (FR-006)

```bash
curl -s http://localhost:8080/embed \
  -H 'Content-Type: application/json' \
  -H 'X-Normalize: false' \
  -d '{"inputs": ["hello"]}' | jq 'length'
```

Expected: `200` with an embedding array — the header is silently ignored; the
model's configured default normalization applies.

## 7. Documentation discoverable (US3, SC-004)

- `docs/api/openai.md` documents the header with values, default, and an
  example, plus the "service-specific extension" note.
- The interactive docs (`http://localhost:8080/docs`) list `X-Normalize` as a
  header parameter on the embeddings path.

## 8. Automated tests

```bash
cargo test --test openai_contract      # header state matrix + errors
cargo test --test openai_integration   # norms, response shape, base64
cargo test                             # full suite (SC-001 regression)
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

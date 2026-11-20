# Quickstart: Model Config Parity

This guide validates feature `008-model-config-parity` end-to-end after
implementation. Contract details: [contracts/tei.md](./contracts/tei.md),
[contracts/config.md](./contracts/config.md); entity definitions:
[data-model.md](./data-model.md).

## Prerequisites

- Rust toolchain (stable, matching CI; currently 1.98)
- `model2vec-rs` 0.5 or later (`Cargo.toml`)
- A local model whose `config.json` declares a `max_length` different from
  `512` (for per-model validation), e.g. a copy of a downloaded model with
  `max_length` edited to `256`, plus an unmodified model
- `curl` and `jq` for verification

## 1. Per-model `max_input_length` (FR-1, FR-2, FR-3)

1. Run the service with a modified model (declares `max_length: 256`) and an
   unmodified model, without setting `MAX_INPUT_LENGTH`:

   ```bash
   cargo run --release -- \
     --model /path/to/model-256 \
     --model minishlab/blink-multilingual-64M \
     --port 8080
   ```

   Expected startup log: one line per model recording its resolved limit
   (`256` and `512` respectively).

2. Query `/info` and verify per-model values:

   ```bash
   curl -s http://localhost:8080/info | jq
   curl -s http://localhost:8080/tei/model-256/info | jq
   ```

   Expected: the root endpoint reports the default model's limit; the
   per-model endpoint reports `256` for the modified model.

3. Verify the explicit override still wins (contracts/config.md row 1):

   ```bash
   MAX_INPUT_LENGTH=128 cargo run --release -- --model /path/to/model-256 --port 8080
   ```

   Expected: startup warns that the override replaces the model's configured
   value; `/info` reports `128`.

## 2. Per-request `normalize` (FR-4)

1. Send a request without the field and confirm unchanged behavior:

   ```bash
   curl -s http://localhost:8080/embed \
     -H 'Content-Type: application/json' \
     -d '{"inputs": ["hello"]}' | jq '.[0] | map(. *.) | add | sqrt'
   ```

   Expected: the norm matches the model's configured default (≈ 1.0 for
   normalized models).

2. Send the same input with `normalize: false` and `normalize: true`:

   ```bash
   curl -s http://localhost:8080/embed \
     -H 'Content-Type: application/json' \
     -d '{"inputs": ["hello"], "normalize": false}' | jq '.[0] | map(. *.) | add | sqrt'
   curl -s http://localhost:8080/embed \
     -H 'Content-Type: application/json' \
     -d '{"inputs": ["hello"], "normalize": true}' | jq '.[0] | map(. *.) | add | sqrt'
   ```

   Expected: the `false` norm differs from the `true` norm (which is ≈ 1.0).

3. Repeat step 2 against `/tei/{model_id}/embed` to confirm both endpoints
   honor the field.

## 3. `/info` normalization metadata (FR-6)

```bash
curl -s http://localhost:8080/info | jq .normalize
```

Expected: a boolean matching the model's `config.json` `normalize` setting
(default `true` when absent).

## 4. OpenAI contract unchanged (FR-5)

```bash
curl -s http://localhost:8080/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{"input": ["hello"], "model": "minishlab/blink-multilingual-64M"}' | jq '.data[0].embedding | length'
```

Expected: standard OpenAI response shape; no `normalize` field accepted or
needed.

## 5. Automated tests

```bash
cargo test                              # all suites
cargo test --test config_unit           # resolution precedence table
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
cargo bench                             # before release; ≤ 10% regression
```

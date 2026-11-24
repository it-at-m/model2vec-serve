# model2vec-serve

Lightweight OpenAI and Text Embedding Inference (TEI) compatible embeddings
server for [model2vec](https://github.com/MinishLab/model2vec) static embedding
models.

## Features

- OpenAI-compatible `POST /v1/embeddings` and `GET /v1/models`
- TEI-compatible `POST /embed` and `GET /info`, plus per-model
  `POST /tei/{model_id}/embed` and `GET /tei/{model_id}/info`
- Optional API key authentication
- Health (`/health`, `/ready`) and metrics (`/metrics`) endpoints
- Interactive OpenAPI documentation at `/docs`
- Structured JSON logs with request correlation IDs
- Small, containerized Rust binary
- One-command two-model local deployment via Docker Compose
- Helm chart for Kubernetes deployment with volume mount support

## Quickstart

Run locally with a Hugging Face model id:

```bash
cargo run --release -- --model minishlab/blink-multilingual-64M --port 8080
```

Request embeddings:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input":"Hello world"}'
```

Serve multiple models in one process:

```bash
cargo run --release -- \
  --model minishlab/blink-multilingual-64M \
  --model minishlab/blink-code-16M \
  --default-model minishlab/blink-multilingual-64M \
  --port 8080
```

Select a model in the request:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input":"def hello(): pass","model":"minishlab/blink-code-16M"}'
```

## TEI per-model endpoints

TEI clients serving multiple models address a model explicitly through
`POST /tei/{model_id}/embed` and `GET /tei/{model_id}/info`. The `{model_id}`
path segment is the model's `--model-alias` or, by default, the last segment of
its identifier (e.g. `minishlab/blink-code-16M` → `blink-code-16M`).
Root `/embed` and `/info` continue to serve the default model.

**Breaking change in 0.5.0**: the `?model=` query parameter on `/embed` and
`/info` was removed. Migrate as follows:

| Before (≤ 0.3.x) | After (0.5.0) |
|------------------|----------------|
| `POST /embed?model=<id>` | `POST /tei/{model_id}/embed` |
| `GET /info?model=<id>` | `GET /tei/{model_id}/info` |
| `POST /embed` (default) | `POST /embed` (default, unchanged) |
| `?model=` present | `400 invalid_request` |

## Configuration

All configuration is passed as command-line arguments:

| Argument | Default | Description |
|----------|---------|-------------|
| `--model` | `minishlab/blink-multilingual-64M` | Hugging Face model id or local path; repeatable |
| `--default-model` | first `--model` | Model to use when a request does not specify one |
| `--model-owner` | `minishlab` | Model publisher or owner shown in `/v1/models` responses |
| `--model-alias` | none | Path identifier alias for a model, as `KEY=ALIAS`; repeatable |
| `--host` | `0.0.0.0` | Bind address |
| `--port` | `8080` | Listen port |
| `--api-key` | none | Enables Bearer token authentication |
| `--max-batch-size` | `256` | Maximum inputs per request |
| `--max-input-length` | `512` | Maximum tokens per input |
| `--log-level` | `info` | Log level |
| `--request-timeout-seconds` | `30` | Per-request timeout |
| `--tls-cert` / `--tls-key` | none | PEM certificate and matching private key file paths; providing both enables HTTPS on the same port (single listener) |

Serve over HTTPS by pointing both TLS options at mounted files — traffic is
then encrypted end to end, all the way to the application inside the
container:

```bash
cargo run --release -- \
  --model minishlab/blink-multilingual-64M \
  --tls-cert /etc/model2vec-serve/tls/tls.crt \
  --tls-key /etc/model2vec-serve/tls/tls.key
```

### Choosing models

Any static model2vec model from the
[minishlab Hugging Face org](https://huggingface.co/minishlab) works. Good
starting points:

| Model | Footprint | Best for |
|-------|-----------|----------|
| `minishlab/blink-multilingual-64M` | ~256 MB | Default — 100+ languages, strongest multilingual quality |
| `minishlab/blink-8M` | ~31 MB | English-only deployments at minimal footprint |
| `minishlab/blink-code-16M` | ~64 MB | Source code (Go, Java, JavaScript, PHP, Python, Ruby) |
| `minishlab/potion-code-16M-v2` | ~33 MB | Superseded by `blink-code-16M` — kept for existing setups |

Models load from the Hugging Face Hub at startup, or from a local path for
air-gapped setups.

## Container

### Build locally

```bash
docker build -t model2vec-serve:latest .
docker run -p 8080:8080 -e MODEL=minishlab/blink-multilingual-64M model2vec-serve:latest
```

Serve multiple models via comma-separated `MODEL`:

```bash
docker run -p 8080:8080 \
  -e MODEL=minishlab/blink-multilingual-64M,minishlab/blink-code-16M \
  -e DEFAULT_MODEL=minishlab/blink-multilingual-64M \
  model2vec-serve:latest
```

### Pull from GitHub Container Registry

Released images are published to GHCR:

```bash
docker pull ghcr.io/it-at-m/model2vec-serve:v0.6.0
docker run -p 8080:8080 -e MODEL=minishlab/blink-multilingual-64M ghcr.io/it-at-m/model2vec-serve:v0.6.0
```

See [docs/deployment/docker.md](docs/deployment/docker.md) for the full release
and tagging strategy.

## Docker Compose

Run a local two-model stack (multilingual + code v2) with a persisted model
cache using the published image:

```bash
docker compose up -d
```

The stack serves `minishlab/blink-multilingual-64M` (default) and
`minishlab/blink-code-16M`. Models download once into `./models` and
survive restarts. Customize via `.env` (see `.env.example`).

See [docs/deployment/compose.md](docs/deployment/compose.md) for the full
guide, including prerequisites, volume mounting, configuration, and
troubleshooting.

## Helm

The Helm chart is maintained in the
[it-at-m/helm-charts](https://github.com/it-at-m/helm-charts) repository and
published to GHCR on every versioned chart change:

```bash
helm install model2vec-serve \
  oci://ghcr.io/it-at-m/helm-charts/model2vec-serve \
  --version 0.6.0 \
  --set models[0]=minishlab/blink-multilingual-64M \
  --set apiKey=your-secret-key
```

Or via the HTTP chart repository:

```bash
helm repo add it-at-m https://it-at-m.github.io/helm-charts
helm repo update
helm install model2vec-serve it-at-m/model2vec-serve \
  --version 0.6.0 \
  --set models={minishlab/blink-multilingual-64M,minishlab/blink-code-16M} \
  --set defaultModel=minishlab/blink-multilingual-64M \
  --set apiKey=your-secret-key
```

See the
[chart README](https://github.com/it-at-m/helm-charts/tree/main/charts/model2vec-serve)
for more options, including volume-mounted models, persistence, ingress, and
end-to-end TLS.

## Development

Run the test suite:

```bash
cargo test
```

Run linting and formatting:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Run benchmarks:

```bash
cargo bench
```

## Repository transfer

This repository was transferred from the personal account
[`freinold`](https://github.com/freinold) to the
[`it-at-m`](https://github.com/it-at-m) organization on 2026-11-19.

- Old GitHub links (`github.com/freinold/model2vec-serve`) redirect here
  automatically.
- The documentation site lives at <https://it-at-m.github.io/model2vec-serve/>;
  the previous `freinold.github.io` URL does **not** redirect.
- Container images and Helm charts published before the transfer remain frozen
  under `ghcr.io/freinold/...`. New releases are published to
  `ghcr.io/it-at-m/model2vec-serve` (image) and
  `oci://ghcr.io/it-at-m/helm-charts/model2vec-serve` (chart).

## Acknowledgements

This service exists because of [model2vec](https://github.com/MinishLab/model2vec)
by [Minish Lab](https://github.com/minishlab). Thank you,
[Stéphan Tulkens](https://github.com/stephantul) and
[Thomas van Dongen](https://github.com/Pringled), for the great project and
for continuing to push static embeddings forward — from the potion series to
the new blink generation.

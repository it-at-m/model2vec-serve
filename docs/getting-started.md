# Getting Started

This guide walks through running `model2vec-serve` locally, calling its
endpoints, and validating the container and Helm chart.

## Prerequisites

- Rust toolchain (stable, 1.98+; the MSRV tracks the toolchain used by CI)
- A model2vec model identifier (e.g. `minishlab/blink-multilingual-64M` for
  multilingual workloads, `minishlab/blink-8M` for English-only deployments, or
  a local model directory)
- Docker (for container validation)
- Helm 3+ and a Kubernetes cluster or local environment such as kind/minikube
  (for Helm validation)

## Run locally

Start the server with a Hugging Face model id:

```bash
cargo run --release -- --model minishlab/blink-multilingual-64M --port 8080
```

You can also set values via environment variables:

```bash
MODEL=minishlab/blink-multilingual-64M PORT=8080 cargo run --release
```

Serve multiple models in one process:

```bash
cargo run --release -- \
  --model minishlab/blink-multilingual-64M \
  --model minishlab/potion-code-16M-v2 \
  --default-model minishlab/blink-multilingual-64M \
  --port 8080
```

## Verify health

```bash
curl http://localhost:8080/health
```

Expected response:

```json
{
  "status": "healthy",
  "ready": true,
  "message": "1 model(s) ready",
  "models": [
    {
      "model_id": "minishlab/blink-multilingual-64M",
      "status": "ready",
      "message": "model loaded"
    }
  ]
}
```

## Request embeddings

### OpenAI-compatible endpoint

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input":"Hello world"}'
```

With multiple models loaded, select a model explicitly:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input":"def hello(): pass","model":"minishlab/potion-code-16M-v2"}'
```

Expected response shape:

```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "index": 0,
      "embedding": [0.0123, -0.0456, "..."]
    }
  ],
  "model": "minishlab/blink-multilingual-64M",
  "usage": {
    "prompt_tokens": 2,
    "total_tokens": 2
  }
}
```

### TEI-compatible endpoint

```bash
curl -X POST http://localhost:8080/embed \
  -H "Content-Type: application/json" \
  -d '{"inputs":"Hello world"}'
```

Expected response:

```json
[[0.0123, -0.0456, "..."]]
```

## Inspect metrics

```bash
curl http://localhost:8080/metrics
```

The response is Prometheus-compatible text with request counters and latency
histograms.

## Enable API key authentication

Start the service with a key:

```bash
cargo run --release -- --model minishlab/blink-multilingual-64M --api-key secret-key
```

An unauthenticated request is rejected:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input":"Hello"}'
```

Expected: `401 Unauthorized`.

A request with the Bearer token succeeds:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer secret-key" \
  -d '{"input":"Hello"}'
```

## Run in Docker

Build the image:

```bash
docker build -t model2vec-serve:latest .
```

Run it:

```bash
docker run -p 8080:8080 \
  -e MODEL=minishlab/blink-multilingual-64M \
  model2vec-serve:latest
```

Serve multiple models:

```bash
docker run -p 8080:8080 \
  -e MODEL=minishlab/blink-multilingual-64M,minishlab/potion-code-16M-v2 \
  -e DEFAULT_MODEL=minishlab/blink-multilingual-64M \
  model2vec-serve:latest
```

Run the same curl checks as in the local section.

## Deploy with Helm

The chart is published from the
[it-at-m/helm-charts](https://github.com/it-at-m/helm-charts) repository.
Install it:

```bash
helm install model2vec-serve \
  oci://ghcr.io/it-at-m/helm-charts/model2vec-serve \
  --set model=minishlab/blink-multilingual-64M \
  --set apiKey=secret-key \
  --set replicaCount=2
```

Install with multiple models:

```bash
helm install model2vec-serve \
  oci://ghcr.io/it-at-m/helm-charts/model2vec-serve \
  --set models={minishlab/blink-multilingual-64M,minishlab/potion-code-16M-v2} \
  --set defaultModel=minishlab/blink-multilingual-64M \
  --set apiKey=secret-key \
  --set replicaCount=2
```

Wait for pods:

```bash
kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=model2vec-serve
```

Port-forward and test:

```bash
kubectl port-forward svc/model2vec-serve 8080:80
curl http://localhost:8080/health
```

See the [Helm](./deployment/helm.md) page for volume-mounted models and all
configuration options.

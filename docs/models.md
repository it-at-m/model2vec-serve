# Models

`model2vec-serve` loads any model in the [model2vec](https://github.com/MinishLab/model2vec)
format: a static embedding model consisting of a `model.safetensors` embedding
matrix plus a tokenizer. Models are downloaded from the Hugging Face Hub at
startup (or read from a local path for air-gapped setups), and multiple models
can be served from a single process.

This guide helps you pick the right model. All models below are served the
same way — the differences are quality, language coverage, memory footprint,
and embedding dimension.

## Quick recommendation

| Use case | Model | Download | Dimension |
|----------|-------|----------|-----------|
| Multilingual (default) | `minishlab/blink-multilingual-64M` | ~256 MB | 256 |
| English only, minimal footprint | `minishlab/blink-8M` | ~31 MB | 256 |
| Source code | `minishlab/potion-code-16M-v2` | ~33 MB | 256 |
| Battle-tested multilingual | `minishlab/potion-multilingual-128M` | ~530 MB | 256 |
| English retrieval / similarity | `minishlab/potion-retrieval-32M` | ~135 MB | 512 |

Download is the on-disk size of the model download — it is **not** a safe
memory budget. `model2vec-rs` allocates the full embedding table at startup
(nothing is memory-mapped), and models published in FP16 are converted to F32
while loading, so their resident memory is roughly **twice** the download
size: `potion-code-16M-v2` occupies ~65 MB despite its ~33 MB download. F32
models (all blink releases and most potion releases) use about the same
amount resident as on disk. For any F32 model the embedding matrix alone is
`vocab × dim × 4` bytes. Size memory limits for the resident value plus
headroom for the tokenizer and request buffers.

## The blink generation (November 2026)

The [blink collection](https://huggingface.co/collections/minishlab/blink) is
the current flagship generation and the official successor to the potion
series. Blink models are initialized from potion distillations and then
trained with contrastive learning and hard-negative fine-tuning, which
recovers much of the quality that static distillation alone loses.

### blink-multilingual-64M

- 100+ languages, custom 250k-tokenizer with no `[UNK]` tokens even for
  Thai and Lao, baked-in language codes.
- Distilled from bge-m3, then contrastively trained on large multilingual
  pair datasets.
- Reported benchmark means (MMTEB, MTEB multilingual v2, 131 tasks):
  **51.60** vs 47.67 for `potion-multilingual-128M`; on non-English tasks
  51.38 vs 47.39; bitext mining 57.03 vs 40.72.
- Roughly **half the memory** of `potion-multilingual-128M` (256 MB vs
  530 MB).

This is the compiled-in default of `model2vec-serve`.

### blink-8M

- English-only, same ~31 MB download as `potion-base-8M` but meaningfully
  better: reported MTEB(eng, v2) mean **55.35** vs 53.30 for
  `potion-base-8M` and 54.02 for `potion-base-32M`; retrieval-oriented
  RTEB(eng) 30.84.
- The best choice when your workload is English-only and memory is tight.

## The potion series

The potion models are pure static distillations — extremely fast and small,
with a long track record. They remain fully supported.

| Model | Languages | Dimension | Download | Notes |
|-------|-----------|-----------|----------|-------|
| `potion-multilingual-128M` | 101 | 256 | ~530 MB | bge-m3 distill; the long-standing multilingual default |
| `potion-code-16M-v2` | code | 256 | ~33 MB | Python, Java, JavaScript, Go, PHP, Ruby; distilled from CodeRankEmbed |
| `potion-base-32M` | en | 512 | ~135 MB | General English |
| `potion-retrieval-32M` | en | 512 | ~135 MB | Retrieval-tuned `potion-base-32M` |
| `potion-base-8M` / `4M` / `2M` | en | 256 / 128 / 64 | 31 MB and below | Tiny models for edge cases and smoke tests |

Notes:

- `potion-code-16M-v2` is still the newest code model — no blink-code exists
  yet. Reported CoIR average: 39.08 (43.36 with a BM25 hybrid). It is
  published in FP16, so resident memory is ~65 MB (see
  [download vs. memory](#quick-recommendation)).
- `potion-retrieval-32M` is the pick for English retrieval workloads when you
  want a pure-distillation model with a long track record.
- The `potion-base-*` tiny models are what this repository's own test suite
  and Helm chart CI use as fixtures.

## Legacy and models to avoid

- `M2V_base_output`, `M2V_multilingual_output`, and related first-generation
  releases — superseded by potion and blink.
- `potion-science-*` — domain experiments, not maintained as general
  embedders.
- The `*-onnx` repositories — ONNX-only, **not loadable** by `model2vec-rs`
  (no `model.safetensors`).

## Choosing a model

Work through these questions in order:

1. **What languages do you need?** Non-English or mixed-language content →
   `blink-multilingual-64M` (or `potion-multilingual-128M` if you prefer the
   battle-tested option). English only → continue.
2. **Is the content source code?** → `potion-code-16M-v2`. For mixed
   repositories, serve it alongside a general model and route requests with
   the `model` field or TEI path aliases.
3. **What is the primary task?** Retrieval/search → `blink-8M` (English) or
   `potion-retrieval-32M` (English, 512d). Clustering, classification,
   similarity → `blink-8M` covers these well at minimal footprint.
4. **What is the memory budget?** Plan for resident memory after loading,
   not the download size — FP16 models double when converted to F32 at load.
   Roughly: ~31 MB (`blink-8M`) → ~65 MB (`potion-code-16M-v2`, ~33 MB
   download) → ~135 MB (32M potions) → ~256 MB (`blink-multilingual-64M`) →
   ~530 MB (`potion-multilingual-128M`). Add headroom for the tokenizer and
   request buffers, and multiply by replicas in Kubernetes.
5. **Risk appetite?** The blink models are new (November 2026) with
   independently reported strong benchmarks; the potion models have been
   deployed at scale since 2024. You can serve both at once and compare on
   your own data before switching.

## Serving multiple models

Pass `--model` repeatedly (or `MODEL` as a comma-separated list). The first
**successfully loaded** model becomes the default used when a request does
not name a model — if a configured model fails to load at startup, the next
one takes over. Set `--default-model` to pin a specific model as the
default; startup then fails if that model cannot be loaded:

```bash
model2vec-serve \
  --model minishlab/blink-multilingual-64M \
  --model minishlab/potion-code-16M-v2
```

OpenAI-style requests select a model via the `model` field; TEI clients use
the per-model paths `/tei/{model_id}/embed` and `/tei/{model_id}/info`. See
[Configuration](./configuration) for aliases and all options.

## Model metadata

- `GET /info` (TEI-style) returns metadata for the **default model** only:
  `model_id`, `max_input_length`, `embedding_dimension`, and pooling mode.
  To inspect a different loaded model, use `GET /tei/{model_id}/info`.
- `GET /v1/models` (OpenAI-style) lists every loaded model with `id`,
  `object`, `created`, and `owned_by`. It deliberately carries no embedding
  dimensions or input limits — use the TEI info endpoints for those.

The `--model-owner` flag (default `minishlab`) controls the publisher shown
in `/v1/models` responses — set it when serving third-party or locally
trained models.

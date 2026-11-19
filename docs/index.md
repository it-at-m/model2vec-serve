---
layout: home

hero:
  name: model2vec-serve
  text: Lightweight Embeddings Server
  tagline: Serve static model2vec embedding models through OpenAI- and TEI-compatible endpoints.
  image:
    src: /model2vec_logo.png
    alt: Model2Vec logo
  actions:
    - theme: brand
      text: Get Started
      link: /getting-started
    - theme: alt
      text: API Reference
      link: /api/openai
    - theme: alt
      text: View on GitHub
      link: https://github.com/it-at-m/model2vec-serve

features:
  - title: OpenAI Compatible
    details: Drop-in replacement for OpenAI's embeddings endpoint with POST /v1/embeddings.
  - title: TEI Compatible
    details: Reuse existing Hugging Face TEI clients with POST /embed and GET /info.
  - title: Optional API Key Auth
    details: Protect embedding endpoints with Authorization Bearer tokens.
  - title: Production Ready
    details: Structured JSON logs, Prometheus metrics, health/readiness probes, and a Helm chart.
---

::: info Repository transfer
This project moved from the personal account `freinold` to the
[it-at-m](https://github.com/it-at-m) organization on 2026-11-19. The previous
`freinold.github.io` documentation URL no longer resolves — update bookmarks to
<https://it-at-m.github.io/model2vec-serve/>.
:::

Built on [model2vec](https://github.com/MinishLab/model2vec) by
[Minish Lab](https://github.com/minishlab) — thank you
[Stéphan Tulkens](https://github.com/stephantul) and
[Thomas van Dongen](https://github.com/Pringled) for the great project and the
continued work on static embedding models.

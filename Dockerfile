# Build stage
FROM rust:1.98-slim@sha256:f47a8de237dcbb0b0ce1099901e60a89728e3d51f24e664b40e947171538ade7 AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    libclang-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches

RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim@sha256:a99cfc517144bc59b1978475ec53b46ecabec7e43635402ee5b77cc54cd1b20a

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/model2vec-serve /usr/local/bin/model2vec-serve

EXPOSE 8080

# The health check probes plain HTTP first and falls back to HTTPS (-k
# accepts self-signed certificates) so it works whether or not the service
# is started with --tls-cert/--tls-key.
HEALTHCHECK --interval=30s --timeout=5s --start-period=300s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/health || curl -fsSk https://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["model2vec-serve"]

FROM rust:1.88.0 AS builder

WORKDIR /app

ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=600 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

COPY starknet_assets/rust/Cargo.toml /app/Cargo.toml
COPY starknet_assets/rust/Cargo.lock /app/Cargo.lock
COPY starknet_assets/rust/src /app/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nf4_starknet_emitter_tool /app/nf4_starknet_emitter_tool

ENTRYPOINT ["/app/nf4_starknet_emitter_tool"]

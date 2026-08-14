FROM rust:1.96-bookworm AS chef
RUN cargo install --locked cargo-chef

FROM chef AS planner
WORKDIR /app
COPY odds-api .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=private \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=private \
    cargo chef cook --release --recipe-path recipe.json
COPY odds-api .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=private \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=private \
    cargo build --release --bin odds-api

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/odds-api /app/server

CMD ["/app/server"]

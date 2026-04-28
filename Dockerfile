FROM rust:bookworm AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin process-bar-3

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 appuser

COPY --from=builder /app/target/release/process-bar-3 /usr/local/bin/process-bar-3
COPY templates ./templates
COPY static ./static
COPY migrations ./migrations

ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080

USER appuser
ENTRYPOINT ["/usr/local/bin/process-bar-3"]

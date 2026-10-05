# syntax=docker/dockerfile:1
# Production image for ouril-server (ADR 0017, ADR 0019). Build from the repo root:
#   docker build -f apps/server/Dockerfile -t ouril-server .
# NEVER enables the `dev-auth` feature (a release build with it fails to compile anyway).

FROM rust:1-bookworm AS build
RUN rustup toolchain install stable --profile minimal && rustup default stable
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p ouril-server \
 && cp target/release/ouril-server /usr/local/bin/ouril-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home ouril
COPY --from=build /usr/local/bin/ouril-server /usr/local/bin/ouril-server
USER ouril
ENV HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=info
EXPOSE 8080
# `ouril-server migrate` applies embedded migrations (Fly release command).
CMD ["ouril-server"]

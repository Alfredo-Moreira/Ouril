# syntax=docker/dockerfile:1
# Dev server image: Rust stable + cargo-watch. Runs ouril-server with hot reload and the
# `dev-auth` feature (POST /v1/auth/dev). Never used for releases: see apps/server/Dockerfile.
FROM rust:1-bookworm

ARG CARGO_WATCH_VERSION=8.5.3
ENV CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=1

# Components/target mirror rust-toolchain.toml; without them rustup would download them at
# runtime on every fresh container (the server itself doesn't need wasm32/clippy/rustfmt).
RUN rustup toolchain install stable --profile minimal \
      --component rustfmt,clippy --target wasm32-unknown-unknown \
 && rustup default stable \
 && curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
 && cargo binstall -y --locked cargo-watch@${CARGO_WATCH_VERSION} \
 && rm -rf /usr/local/cargo/registry /usr/local/cargo/git

WORKDIR /workspace
EXPOSE 8080
# Watch only the crates the server depends on, so web/docs edits don't trigger rebuilds.
CMD ["cargo", "watch", "--why", \
     "-w", "apps/server", "-w", "core/engine", "-w", "core/protocol", "-w", "core/sync", \
     "-w", "Cargo.toml", \
     "-x", "run -p ouril-server --features dev-auth"]

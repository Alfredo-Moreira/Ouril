# syntax=docker/dockerfile:1
# Homelab image for ouril-relay (ADR 0032). Built for linux/amd64 and linux/arm64 by
# deploy/homelab/build-and-push.sh, from the repo root:
#   docker buildx build -f apps/relay/Dockerfile --build-context homelab=deploy/homelab \
#     --platform linux/amd64,linux/arm64 .
#
# - Cross-compiles on the build machine (no emulation): the relay is pure Rust, so only a
#   cross linker is needed.
# - The runtime is distroless (no shell, no package manager) and runs as a non-root user.
# - Like the `home-lab` repo's images, deploy/homelab/.env is baked in (/app/.env) when it
#   exists, so the NAS needs nothing but the compose file. It reaches this image only
#   through the `homelab` build context: the main context excludes it (.dockerignore), so it
#   can never end up in another image. Never push this image to a public registry.

FROM --platform=$BUILDPLATFORM rust:1-bookworm AS build
ARG TARGETARCH
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      gcc-aarch64-linux-gnu libc6-dev-arm64-cross gcc-x86-64-linux-gnu libc6-dev-amd64-cross \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-gnu > /target ;; \
      arm64) echo aarch64-unknown-linux-gnu > /target ;; \
      *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && rustup target add "$(cat /target)"
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
# Both architectures build at the same time and share the crate registry cache: `locked` makes
# them take turns, or two cargo processes unpack the same crate at once and one fails with
# "File exists". Each architecture has its own target cache.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=ouril-relay-cargo-registry,sharing=locked \
    --mount=type=cache,target=/src/target,id=ouril-relay-target-${TARGETARCH} \
    cargo build --release --locked -p ouril-relay --target "$(cat /target)" \
 && cp "target/$(cat /target)/release/ouril-relay" /ouril-relay

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /ouril-relay /ouril-relay
# VERSION is listed so the wildcard always matches; CI builds have no .env.
COPY --from=homelab VERSION .env* /app/
ENV PORT=8090 \
    RUST_LOG=info \
    LOG_FORMAT=json
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD ["/ouril-relay", "healthcheck"]
ENTRYPOINT ["/ouril-relay"]

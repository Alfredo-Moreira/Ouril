# syntax=docker/dockerfile:1
# Images for Ouril's homelab services (ADR 0032): the live-game relay (`ouril-relay`) and the
# scheduled jobs (`ouril-jobs`). Built for linux/amd64 and linux/arm64 by
# deploy/homelab/build-and-push.sh, from the repo root:
#   docker buildx build -f deploy/homelab/Dockerfile --build-context homelab=deploy/homelab \
#     --build-arg PACKAGE=ouril-relay --build-arg ENV_KEYS='RELAY_API_KEY|…' \
#     --platform linux/amd64,linux/arm64 .
#
# - Cross-compiles on the build machine (no emulation); only cross linkers are needed.
# - The runtime is distroless (no shell, no package manager) and runs as a non-root user.
# - Like the `home-lab` repo's images, settings from deploy/homelab/.env are baked in
#   (/app/.env), but only the keys this service uses (ENV_KEYS): one image never carries
#   another's secrets. The .env reaches this build only through the `homelab` build context;
#   the main context excludes it (.dockerignore). Never push these images to a public registry.

ARG PACKAGE=ouril-relay

FROM --platform=$BUILDPLATFORM rust:1-bookworm AS build
ARG TARGETARCH
ARG PACKAGE
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
# "File exists". Each architecture and package has its own target cache.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=ouril-homelab-cargo-registry,sharing=locked \
    --mount=type=cache,target=/src/target,id=ouril-homelab-target-${PACKAGE}-${TARGETARCH} \
    cargo build --release --locked -p "$PACKAGE" --target "$(cat /target)" \
 && cp "target/$(cat /target)/release/$PACKAGE" /service

# Only this service's keys from deploy/homelab/.env (when there is one; CI builds have none).
FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS settings
ARG ENV_KEYS=""
COPY --from=homelab VERSION .env* /in/
RUN mkdir /out && cp /in/VERSION /out/ \
 && if [ -f /in/.env ] && [ -n "$ENV_KEYS" ]; then \
      grep -E "^(${ENV_KEYS})=" /in/.env > /out/.env || true; \
    fi

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /service /service
COPY --from=settings /out/ /app/
ENV RUST_LOG=info \
    LOG_FORMAT=json
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/service", "healthcheck"]
ENTRYPOINT ["/service"]

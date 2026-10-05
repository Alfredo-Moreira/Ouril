# syntax=docker/dockerfile:1
# Ouril toolbox: Rust (stable + wasm32), wasm-pack, wasm-bindgen-cli, binaryen (wasm-opt), sqlx-cli,
# cargo-watch, just, Node LTS + pnpm. Used by `just` recipes, CI and the Dev Container
# (ADR 0017). The Android NDK + cargo-ndk are OPTIONAL: build with
#   docker compose build --build-arg INSTALL_ANDROID_NDK=true toolbox
FROM node:lts-bookworm-slim AS node

FROM rust:1-bookworm

ARG WASM_BINDGEN_VERSION=0.2.129
ARG WASM_PACK_VERSION=0.15.0
ARG SQLX_CLI_VERSION=0.9.0
ARG CARGO_WATCH_VERSION=8.5.3
ARG JUST_VERSION=1.58.0
ARG BINARYEN_VERSION=133
ARG PNPM_VERSION=12.9.1
ARG INSTALL_ANDROID_NDK=false
ARG ANDROID_NDK_VERSION=r27c

ENV CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=0

# The repo pins `channel = "stable"` in rust-toolchain.toml; install that channel by name
# so rustup never downloads a toolchain at `docker compose run` time.
RUN rustup toolchain install stable --profile minimal \
      --component rustfmt,clippy --target wasm32-unknown-unknown \
 && rustup default stable

# cargo-binstall fetches prebuilt binaries where they exist, otherwise builds from source.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
 && cargo binstall -y --locked \
      wasm-pack@${WASM_PACK_VERSION} \
      wasm-bindgen-cli@${WASM_BINDGEN_VERSION} \
      cargo-watch@${CARGO_WATCH_VERSION} \
      just@${JUST_VERSION} \
 && rm -rf /usr/local/cargo/registry /usr/local/cargo/git

# binaryen (wasm-opt) for production-size WASM, pinned and checksum-verified. `just
# bindings-wasm` runs it after wasm-pack (wasm-pack's own download stays off, so builds work
# offline once the image exists).
RUN arch="$(uname -m)" \
 && case "$arch" in x86_64|aarch64) ;; *) echo "binaryen: unsupported arch $arch" >&2; exit 1;; esac \
 && f="binaryen-version_${BINARYEN_VERSION}-${arch}-linux.tar.gz" \
 && base="https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}" \
 && curl -fsSL -o "/tmp/$f" "$base/$f" \
 && curl -fsSL -o "/tmp/$f.sha256" "$base/$f.sha256" \
 && echo "$(cut -d' ' -f1 "/tmp/$f.sha256")  /tmp/$f" | sha256sum -c - \
 && mkdir -p /opt/binaryen && tar -xzf "/tmp/$f" -C /opt/binaryen --strip-components=1 \
 && rm -f "/tmp/$f" "/tmp/$f.sha256"
ENV PATH="/opt/binaryen/bin:${PATH}"

# sqlx-cli: Postgres + rustls only (binstall can't select features, so build it).
RUN cargo install --locked sqlx-cli@${SQLX_CLI_VERSION} \
      --no-default-features --features postgres,rustls \
 && rm -rf /usr/local/cargo/registry /usr/local/cargo/git

# Node LTS + pnpm (for wasm-pack output checks and TS tooling such as protocol-ts).
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -sf /usr/local/lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
 && ln -sf /usr/local/lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
 && npm install -g pnpm@${PNPM_VERSION} \
 && node --version && pnpm --version

# Optional: Android NDK + cargo-ndk (only needed later for apps/android, ADR 0018).
RUN if [ "$INSTALL_ANDROID_NDK" = "true" ]; then \
      apt-get update && apt-get install -y --no-install-recommends unzip && rm -rf /var/lib/apt/lists/* \
      && curl -fsSL -o /tmp/ndk.zip "https://dl.google.com/android/repository/android-ndk-${ANDROID_NDK_VERSION}-linux.zip" \
      && unzip -q /tmp/ndk.zip -d /opt && rm /tmp/ndk.zip \
      && mv /opt/android-ndk-${ANDROID_NDK_VERSION} /opt/android-ndk \
      && rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android \
      && cargo binstall -y --locked cargo-ndk \
      && rm -rf /usr/local/cargo/registry /usr/local/cargo/git; \
    fi
ENV ANDROID_NDK_HOME=/opt/android-ndk

WORKDIR /workspace
CMD ["bash"]

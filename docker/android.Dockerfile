# syntax=docker/dockerfile:1
# Ouril Android build image: JDK, the Android SDK (platform, build tools, NDK), Rust with the
# Android targets and cargo-ndk, and just. It builds the core's Android libraries and Kotlin
# bindings (`just bindings-android`, ADR 0015) and the app itself (`just android-build`,
# `just android-test`), the same way locally and in CI (ADR 0017).
#
# linux/amd64 only: the NDK and the SDK build tools (aapt2) ship for x86_64 Linux. On an arm64
# Mac, Docker runs it under emulation, so the first build is slow; later ones use the caches in
# the compose volumes. Android Studio and the emulator stay on the host.
FROM --platform=linux/amd64 eclipse-temurin:21-jdk-noble

ARG CMDLINE_TOOLS_BUILD=16111833
ARG ANDROID_PLATFORM=android-37.0
ARG BUILD_TOOLS=37.0.0
ARG NDK_VERSION=30.0.16248370
ARG CARGO_NDK_VERSION=4.1.2
ARG JUST_VERSION=1.58.0

ENV ANDROID_HOME=/opt/android-sdk \
    ANDROID_SDK_ROOT=/opt/android-sdk \
    ANDROID_NDK_HOME=/opt/android-sdk/ndk/${NDK_VERSION} \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_TERM_COLOR=never \
    CARGO_INCREMENTAL=0
ENV PATH="${CARGO_HOME}/bin:${ANDROID_HOME}/cmdline-tools/latest/bin:${ANDROID_HOME}/platform-tools:${PATH}"

RUN apt-get update \
 && apt-get install -y --no-install-recommends unzip curl git build-essential ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Android SDK: command-line tools, then the platform, build tools and NDK.
RUN mkdir -p ${ANDROID_HOME}/cmdline-tools \
 && curl -fsSL -o /tmp/tools.zip \
      "https://dl.google.com/android/repository/commandlinetools-linux-${CMDLINE_TOOLS_BUILD}_latest.zip" \
 && unzip -q /tmp/tools.zip -d ${ANDROID_HOME}/cmdline-tools \
 && mv ${ANDROID_HOME}/cmdline-tools/cmdline-tools ${ANDROID_HOME}/cmdline-tools/latest \
 && rm /tmp/tools.zip \
 && yes | sdkmanager --licenses >/dev/null \
 && sdkmanager --install "platforms;${ANDROID_PLATFORM}" "build-tools;${BUILD_TOOLS}" \
      "platform-tools" "ndk;${NDK_VERSION}" >/dev/null

# Rust (the repo's rust-toolchain.toml pins the stable channel), the Android targets and the
# host target (the JVM unit tests load the host library), cargo-ndk and just.
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
 && rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android \
 && curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
 && cargo binstall -y --locked cargo-ndk@${CARGO_NDK_VERSION} just@${JUST_VERSION} \
 && rm -rf ${CARGO_HOME}/registry ${CARGO_HOME}/git

WORKDIR /workspace
CMD ["bash"]

# syntax=docker/dockerfile:1
# Web dev image: Node LTS + pnpm (pinned). Used by the `web` compose service (ADR 0017).
FROM node:lts-bookworm-slim
ARG PNPM_VERSION=12.9.1
RUN npm install -g pnpm@${PNPM_VERSION} && pnpm --version
# The pnpm store lives inside the node_modules named volume (same filesystem: hard links).
ENV CI=true
WORKDIR /workspace

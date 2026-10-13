#!/usr/bin/env bash
# Build Ouril's homelab images for linux/amd64 and linux/arm64 and push them to the private
# registry, then point docker-compose.homelab.yml at the new tag (ADR 0032). The same flow as
# the home-lab repo's scripts/build-and-push.sh.
#
#   deploy/homelab/build-and-push.sh [tag]     (tag defaults to deploy/homelab/VERSION)
#
# Then paste deploy/homelab/docker-compose.homelab.yml into the UGOS Docker UI.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "${HERE}/../.." && pwd)"
ENV_FILE="${HERE}/.env"
COMPOSE_FILE="${HERE}/docker-compose.homelab.yml"

# Read one KEY=value from .env without executing the file.
env_value() {
  grep -E "^${1}=" "${ENV_FILE}" 2>/dev/null | tail -n 1 | cut -d= -f2- || true
}

echo "==> Checking ${ENV_FILE} (baked into the relay image)"
if [ ! -f "${ENV_FILE}" ]; then
  echo "error: ${ENV_FILE} not found. Copy .env.example to .env and fill it in." >&2
  exit 1
fi
REGISTRY="$(env_value REGISTRY)"
if ! [[ "${REGISTRY}" =~ ^[A-Za-z0-9.-]+:[0-9]+$ ]]; then
  echo "error: set REGISTRY=host:port in .env (e.g. REGISTRY=192.168.1.10:5000)" >&2
  exit 1
fi
API_KEY="$(env_value RELAY_API_KEY)"
TOKEN_SECRET="$(env_value RELAY_TOKEN_SECRET)"
for pair in "RELAY_API_KEY:${API_KEY}" "RELAY_TOKEN_SECRET:${TOKEN_SECRET}"; do
  name="${pair%%:*}"
  value="${pair#*:}"
  if [ "${#value}" -lt 32 ]; then
    echo "error: ${name} must be at least 32 characters (openssl rand -hex 32)" >&2
    exit 1
  fi
done
if [ "${API_KEY}" = "${TOKEN_SECRET}" ]; then
  echo "error: RELAY_API_KEY and RELAY_TOKEN_SECRET must differ" >&2
  exit 1
fi
ORIGINS="$(env_value ALLOWED_ORIGINS)"
if [ -z "${ORIGINS}" ]; then
  echo "error: set ALLOWED_ORIGINS in .env (e.g. https://ouril.vercel.app)" >&2
  exit 1
fi
IFS=',' read -ra origin_list <<< "${ORIGINS}"
for o in "${origin_list[@]}"; do
  o="$(echo "${o}" | tr -d '[:space:]')"
  case "${o}" in
    https://*) ;;
    *) echo "error: ALLOWED_ORIGINS must be https:// origins: ${o}" >&2; exit 1 ;;
  esac
done

VERSION="$(tr -d '[:space:]' < "${HERE}/VERSION")"
TAG="${1:-${VERSION}}"
IMAGE="${REGISTRY}/ouril-relay"

echo "==> Setting up a multi-platform builder"
# The registry speaks plain HTTP on the LAN: tell BuildKit, for this registry only.
BUILDKITD_CONFIG="$(mktemp)"
trap 'rm -f "${BUILDKITD_CONFIG}"' EXIT
printf '[registry."%s"]\n  http = true\n  insecure = true\n' "${REGISTRY}" > "${BUILDKITD_CONFIG}"
docker buildx inspect ouril-homelab-builder &>/dev/null \
  && docker buildx rm ouril-homelab-builder
docker buildx create --name ouril-homelab-builder --config "${BUILDKITD_CONFIG}" --use >/dev/null

echo "==> Building ${IMAGE}:${TAG} (linux/amd64, linux/arm64)"
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --file "${ROOT}/apps/relay/Dockerfile" \
  --build-context "homelab=${HERE}" \
  --push \
  -t "${IMAGE}:${TAG}" \
  -t "${IMAGE}:latest" \
  "${ROOT}"

echo "==> Pointing docker-compose.homelab.yml at ${IMAGE}:${TAG}"
# Matches any registry, so changing REGISTRY in .env also moves the compose file.
tmp="$(mktemp)"
sed -E "s|^( *image: *)[^ ]+/ouril-relay:[^ ]+|\\1${IMAGE}:${TAG}|" "${COMPOSE_FILE}" > "${tmp}"
mv "${tmp}" "${COMPOSE_FILE}"

echo "==> Done. Paste deploy/homelab/docker-compose.homelab.yml into the UGOS Docker UI."

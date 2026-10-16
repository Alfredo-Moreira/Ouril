#!/usr/bin/env bash
# Build Ouril's homelab images (ouril-relay, ouril-jobs) for linux/amd64 and linux/arm64 and push
# them to the private registry, then point docker-compose.homelab.yml at the new tag (ADR 0032).
# The same flow as the home-lab repo's scripts/build-and-push.sh.
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

echo "==> Checking ${ENV_FILE} (each image gets only its own keys)"
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

JOBS_URL="$(env_value JOBS_URL)"
JOBS_TOKEN="$(env_value JOBS_TOKEN)"
case "${JOBS_URL}" in
  https://*) ;;
  *) echo "error: set JOBS_URL to the server's https:// address (e.g. https://ouril.vercel.app)" >&2; exit 1 ;;
esac
if [ "${#JOBS_TOKEN}" -lt 32 ]; then
  echo "error: JOBS_TOKEN must be at least 32 characters (openssl rand -hex 32)" >&2
  exit 1
fi
for other in "${API_KEY}" "${TOKEN_SECRET}"; do
  if [ "${JOBS_TOKEN}" = "${other}" ]; then
    echo "error: JOBS_TOKEN must differ from the relay's secrets" >&2
    exit 1
  fi
done

VERSION="$(tr -d '[:space:]' < "${HERE}/VERSION")"
TAG="${1:-${VERSION}}"

# Each service and the .env keys its image may carry.
SERVICES=(ouril-relay ouril-jobs)
keys_for() {
  case "$1" in
    ouril-relay) echo 'RELAY_API_KEY|RELAY_TOKEN_SECRET|ALLOWED_ORIGINS|MAX_CONNECTIONS|MAX_CONNECTIONS_PER_IP|UPGRADES_PER_MINUTE_PER_IP' ;;
    ouril-jobs) echo 'JOBS_URL|JOBS_TOKEN|JOBS_INTERVAL_SECS|JOBS_LIST' ;;
  esac
}

echo "==> Setting up a multi-platform builder"
# The registry speaks plain HTTP on the LAN: tell BuildKit, for this registry only.
BUILDKITD_CONFIG="$(mktemp)"
trap 'rm -f "${BUILDKITD_CONFIG}"' EXIT
printf '[registry."%s"]\n  http = true\n  insecure = true\n' "${REGISTRY}" > "${BUILDKITD_CONFIG}"
docker buildx inspect ouril-homelab-builder &>/dev/null \
  && docker buildx rm ouril-homelab-builder
docker buildx create --name ouril-homelab-builder --config "${BUILDKITD_CONFIG}" --use >/dev/null

tmp="$(mktemp)"
cp "${COMPOSE_FILE}" "${tmp}"
for service in "${SERVICES[@]}"; do
  image="${REGISTRY}/${service}"
  echo "==> Building ${image}:${TAG} (linux/amd64, linux/arm64)"
  docker buildx build \
    --platform linux/amd64,linux/arm64 \
    --file "${HERE}/Dockerfile" \
    --build-context "homelab=${HERE}" \
    --build-arg "PACKAGE=${service}" \
    --build-arg "ENV_KEYS=$(keys_for "${service}")" \
    --push \
    -t "${image}:${TAG}" \
    -t "${image}:latest" \
    "${ROOT}"
  # Matches any registry, so changing REGISTRY in .env also moves the compose file.
  sed -E "s|^( *image: *)[^ ]+/${service}:[^ ]+|\\1${image}:${TAG}|" "${tmp}" > "${tmp}.next"
  mv "${tmp}.next" "${tmp}"
done

echo "==> Pointing docker-compose.homelab.yml at ${TAG}"
mv "${tmp}" "${COMPOSE_FILE}"

echo "==> Done. Paste deploy/homelab/docker-compose.homelab.yml into the UGOS Docker UI."

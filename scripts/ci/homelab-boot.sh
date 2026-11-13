#!/usr/bin/env bash
# Boot checks for the homelab images, run by CI (homelab job) and scripts/check.sh against
# images tagged ouril-relay:ci and ouril-jobs:ci. Nothing here pushes an image anywhere.
set -euo pipefail
port=${RELAY_CHECK_PORT:-18090} # not 8090, so a local `just relay` can keep running
log=$(mktemp)
cleanup() { docker rm -f ouril-relay-check >/dev/null 2>&1 || true; rm -f "$log"; }
trap cleanup EXIT
fail() { echo "::error::$*" >&2; exit 1; }

echo "relay: boots read-only, non-root, healthy"
docker run -d --name ouril-relay-check --read-only --cap-drop ALL -p "127.0.0.1:$port:8090" \
  -e RELAY_API_KEY="$(openssl rand -hex 32)" \
  -e RELAY_TOKEN_SECRET="$(openssl rand -hex 32)" \
  -e ALLOWED_ORIGINS=https://ouril.vercel.app \
  ouril-relay:ci >/dev/null
for _ in $(seq 1 20); do curl -sf "localhost:$port"/healthz >/dev/null && break; sleep 1; done
curl -sf "localhost:$port"/healthz >/dev/null || fail "relay never became healthy"
test "$(curl -s -o /dev/null -w '%{http_code}' "localhost:$port"/admin)" = 404 || fail "relay serves /admin"
docker exec ouril-relay-check /service healthcheck || fail "relay healthcheck failed"
test "$(docker inspect ouril-relay-check --format '{{.Config.User}}')" != "" || fail "relay runs as root"

echo "jobs: refuses to start without its settings, unhealthy before a round"
# The service logs to stdout, so capture both streams.
if docker run --rm ouril-jobs:ci >"$log" 2>&1; then fail "ouril-jobs started without JOBS_URL"; fi
grep -q "JOBS_URL is required" "$log" || { cat "$log"; fail "ouril-jobs failed for another reason"; }
if docker run --rm -e JOBS_URL=https://example.invalid -e JOBS_TOKEN="$(openssl rand -hex 32)" \
    ouril-jobs:ci healthcheck >/dev/null 2>&1; then
  fail "ouril-jobs is healthy before any round"
fi

echo "images hold no .env"
for p in ouril-relay ouril-jobs; do
  id=$(docker create "$p:ci")
  if docker cp "$id:/app/.env" - >/dev/null 2>&1; then docker rm "$id" >/dev/null; fail "$p contains a .env"; fi
  docker rm "$id" >/dev/null
done
echo "homelab images OK"

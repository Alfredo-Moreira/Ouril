#!/usr/bin/env bash
# Run CI's checks locally, through Docker like everything else (ADR 0017), so a failure shows up
# before a push instead of after it. The git hooks in .githooks call this (`just hooks` turns
# them on); `just ci` runs all of it.
#
#   scripts/check.sh pre-commit   fast checks for the staged files' areas
#   scripts/check.sh pre-push     fast and slow checks for the pushed commits' areas (git's
#                                 pre-push input on stdin; without it, the diff from origin/main)
#   scripts/check.sh all          every check, as CI runs them, including both image architectures
#
# Each check runs only when a file in its area changed. Checks read the working tree, so in a
# commit with unstaged changes they see those too. Skip the hooks once with `git commit
# --no-verify` / `git push --no-verify`. Mirrors .github/workflows/ci.yml and the deploy
# assembly in deploy-web.yml: change them together.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode=${1:-all}
case "$mode" in pre-commit | pre-push | all) ;; *)
  echo "usage: scripts/check.sh pre-commit | pre-push | all" >&2
  exit 2
  ;;
esac

# Plain output: no ANSI codes in logs (and never grep cargo output for errors; exit codes decide).
tb="docker compose run --rm -T -e CARGO_TERM_COLOR=never toolbox"
web="docker compose run --rm -T web"

# --- What changed -------------------------------------------------------------------------------
zero=0000000000000000000000000000000000000000
files=""
case "$mode" in
  pre-commit)
    files=$(git diff --cached --name-only)
    ;;
  pre-push)
    if [ -t 0 ]; then
      files=$(git diff --name-only "$(git merge-base origin/main HEAD)" HEAD)
    else
      while read -r _local_ref local_sha _remote_ref remote_sha; do
        [ "$local_sha" = "$zero" ] && continue # deleting a remote branch
        base=$remote_sha
        # A new branch, or a remote commit we don't have (a force-push): compare with main.
        if [ "$base" = "$zero" ] || ! git cat-file -e "$base^{commit}" 2>/dev/null; then
          base=$(git merge-base origin/main "$local_sha")
        fi
        files="$files"$'\n'"$(git diff --name-only "$base" "$local_sha")"
      done
    fi
    ;;
esac

changed() { # changed <extended regex>: did any file in this run match?
  [ "$mode" = all ] && return 0
  printf '%s\n' "$files" | grep -Eq "$1"
}

rust='(\.rs$|(^|/)Cargo\.(toml|lock)$|^rust-toolchain\.toml$|^apps/server/migrations/|^core/test-vectors/)'
webapp='^(apps/web/|shared/|package\.json$|pnpm-(lock|workspace)\.yaml$)'
core='^(core/|Cargo\.lock$)'
docs='(\.md$|^docs/|^\.claude/skills/|^core/test-vectors/)'
shell='(\.sh$|^\.githooks/)'
compose='^(compose\.yaml$|docker/|deploy/homelab/docker-compose)'
homelab='^(deploy/homelab/|apps/relay/|apps/jobs/|scripts/ci/homelab-boot\.sh$|Cargo\.(toml|lock)$)'
deploy='((^|/)Cargo\.(toml|lock)$|^\.github/workflows/deploy-web\.yml$|^scripts/ci/assemble-api\.sh$|^apps/server/Dockerfile\.vercel$)'
server_image='^apps/server/(Dockerfile|Cargo\.toml)$'
vercel_json='^apps/web/vercel\.json$'

if [ "$mode" != all ] && [ -z "$(printf '%s' "$files" | tr -d '[:space:]')" ]; then
  exit 0
fi

# --- Running ------------------------------------------------------------------------------------
current=""
started=$(date +%s)
step() {
  current=$1
  printf '\n\033[1m==> %s\033[0m\n' "$1"
}
report() {
  local status=$?
  if [ "$status" -ne 0 ] && [ -n "$current" ]; then
    printf '\n\033[31mFAILED: %s\033[0m (scripts/check.sh %s)\n' "$current" "$mode" >&2
    if [ "$mode" != all ]; then echo "Fix it, or skip the hook once with --no-verify." >&2; fi
  fi
}
trap report EXIT

ensure_env() { [ -f .env ] || cp .env.example .env; }
wasm_built=false
ensure_wasm() { # The web app imports core/wasm/pkg; rebuild it (once) when the core changed.
  if ! $wasm_built && { [ ! -f core/wasm/pkg/package.json ] || changed "$core"; }; then
    wasm_built=true
    step "WASM engine (wasm-pack, wasm-opt)"
    $tb wasm-pack build core/wasm --target web --out-dir pkg --out-name ouril_wasm --release
    $tb wasm-opt -Oz --strip-debug core/wasm/pkg/ouril_wasm_bg.wasm -o core/wasm/pkg/ouril_wasm_bg.wasm
  fi
}
pnpm_install="pnpm install --frozen-lockfile --reporter=silent"

ensure_env
slow=false
[ "$mode" != pre-commit ] && slow=true

# Fast: what a commit can break on its own.
if changed "$docs"; then
  step "Docs check and test vector bookkeeping"
  python3 .claude/skills/docs-audit/scripts/check_docs.py .
  python3 .claude/skills/test-vectors/scripts/check_vectors.py .
fi

if changed "$shell"; then
  step "Shellcheck"
  # shellcheck disable=SC2046 # word splitting of the file list is intended
  docker run --rm -v "$PWD:/mnt" -w /mnt koalaman/shellcheck:stable \
    $(git ls-files '*.sh' '.githooks/*')
fi

if changed "$compose"; then
  step "Compose files are valid"
  docker compose config --quiet
  docker compose -f deploy/homelab/docker-compose.homelab.yml config --quiet
fi

if changed "$rust"; then
  step "rustfmt"
  $tb cargo fmt --all -- --check
  step "Clippy (all features, warnings are errors)"
  $tb cargo clippy --workspace --all-targets --all-features -- -D warnings
fi

if changed "$webapp"; then
  ensure_wasm
  step "Web: ESLint, typecheck, Prettier"
  $web sh -c "$pnpm_install && pnpm --filter web lint && pnpm --filter web typecheck && pnpm --filter web format:check"
fi

if changed "$deploy"; then
  step "Deploy assembly: the server's workspace loads with every member"
  scripts/ci/assemble-api.sh .check/api
  $tb cargo metadata --no-deps --offline --locked --format-version 1 \
    --manifest-path .check/api/Cargo.toml >/dev/null
fi

# Slow: tests, audits and images, before a push.
if $slow && changed "$rust"; then
  step "Rust tests (all features, with Postgres)"
  docker compose up -d --wait db
  $tb cargo test --workspace --all-features
  step "Server tests without dev-auth"
  $tb cargo test -p ouril-server
  step "cargo audit (RustSec)"
  $tb sh -c "cargo binstall -y --locked --quiet cargo-audit && cargo audit"
fi

if $slow && { changed "$webapp" || changed "$core"; }; then
  ensure_wasm
  step "Web: tests and production build"
  $web sh -c "$pnpm_install && pnpm --filter web test && pnpm --filter web build"
  step "pnpm audit (prod, high and critical)"
  $web pnpm audit --prod --audit-level high
fi

if $slow && changed "$vercel_json"; then
  step "vercel.json against Vercel's schema"
  docker run --rm -v "$PWD/apps/web/vercel.json:/vercel.json:ro" python:3.12-slim sh -c '
    pip install --quiet --disable-pip-version-check --root-user-action=ignore jsonschema &&
    python3 - <<"EOF"
import json, sys, urllib.request
import jsonschema
schema = json.load(urllib.request.urlopen("https://openapi.vercel.sh/vercel.json"))
errors = list(jsonschema.Draft7Validator(schema).iter_errors(json.load(open("/vercel.json"))))
for e in errors:
    print(f"apps/web/vercel.json: {list(e.path)}: {e.message}")
sys.exit(1 if errors else 0)
EOF'
fi

if $slow && changed "$server_image"; then
  step "Release server image has no dev sign-in route"
  docker build -q -f apps/server/Dockerfile -t ouril-server:ci . >/dev/null
  id=$(docker create ouril-server:ci)
  if docker cp "$id:/usr/local/bin/ouril-server" - | grep -a -q "/auth/dev"; then
    docker rm "$id" >/dev/null
    echo "the release binary contains the dev-auth route" >&2
    exit 1
  fi
  docker rm "$id" >/dev/null
fi

if $slow && changed "$homelab"; then
  for p in ouril-relay ouril-jobs; do
    if [ "$mode" = all ]; then
      # Both architectures at once, like CI and build-and-push.sh (the NAS is arm64).
      step "Homelab image $p (amd64 and arm64)"
      docker buildx build -q -f deploy/homelab/Dockerfile --build-context homelab=deploy/homelab \
        --build-arg PACKAGE=$p --platform linux/amd64,linux/arm64 . >/dev/null
    fi
    step "Homelab image $p (this machine's architecture)"
    docker buildx build -q --load -f deploy/homelab/Dockerfile --build-context homelab=deploy/homelab \
      --build-arg PACKAGE=$p -t $p:ci . >/dev/null
  done
  step "Homelab images boot"
  scripts/ci/homelab-boot.sh
fi

current=""
printf '\n\033[32mscripts/check.sh %s: all checks passed\033[0m (%ss)\n' "$mode" "$(($(date +%s) - started))"

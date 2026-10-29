#!/usr/bin/env bash
# Assemble the Rust workspace that Vercel builds the server from (deploy-web.yml) into $1.
# Cargo loads every workspace member even to build one crate, so each member's sources must be
# here, even those not deployed on Vercel (the homelab services). scripts/check.sh runs this too
# and then `cargo metadata` on the result, so a missing member fails at commit time, not deploy.
set -euo pipefail
out=${1:?usage: assemble-api.sh <out-dir>}
rm -rf "$out"
mkdir -p "$out/apps"
cp Cargo.toml Cargo.lock "$out/"
rsync -a --exclude target --exclude 'wasm/pkg' core "$out/"
rsync -a --exclude target apps/server apps/relay apps/jobs "$out/apps/"
cp apps/server/Dockerfile.vercel "$out/Dockerfile.vercel"

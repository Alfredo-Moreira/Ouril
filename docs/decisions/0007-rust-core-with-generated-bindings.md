# 0007. Shared Rust core with generated bindings

- **Status:** Accepted
- **Date:** 2026-10-03
- **Supersedes:** [0002](0002-typescript-monorepo.md)

## Context
Ouril will ship **native** apps (Swift on iOS, Kotlin on Android), a web app (TypeScript), and a server. The rules engine and AI must behave **identically** everywhere: on-device for offline play, and on the server to validate online moves. Writing the rules three or four times would mean three or four sets of bugs.

## Decision
- Write the **rules engine, AI and protocol types once in Rust** (`core/`).
- Generate platform bindings:
  - **UniFFI** → Swift (packaged as an XCFramework) and Kotlin (with `.so` libraries built via `cargo-ndk`).
  - **wasm-bindgen** → WebAssembly + TypeScript types for the web.
  - The server is Rust and uses the crates directly.
- Keep the core **pure**: no networking, storage or platform APIs. Those stay native in each app.
- Keep **language-neutral JSON test vectors** in `core/test-vectors/`, run in Rust and through every binding in CI.
- Use a polyglot monorepo with Cargo, Xcode/SPM, Gradle and pnpm, orchestrated with `just`. See [monorepo.md](../architecture/monorepo.md).

## Alternatives considered
- **Kotlin Multiplatform:** native on Android with good tooling, but Swift interop feels less natural, the web output is larger, and the AI would be slower.
- **TypeScript core in embedded JS engines** (JavaScriptCore, QuickJS): awkward to debug, and too slow for the AI search.
- **C++ core:** the bindings have to be written by hand, and it's less safe.
- **Rules written separately per platform with shared tests:** viable for the rules alone, but the AI would also need writing three times.

## Consequences
- One implementation of the rules and AI. It's fast on low-end phones, and the WASM build stays small.
- The team needs Rust, but only for a small, pure core and the server.
- The build pipeline is more complex: cross-compiling for iOS and Android ABIs, packaging the XCFramework, building WASM. This is automated with `just` and CI.
- Everything runs offline on device. Rust does not imply online-only.

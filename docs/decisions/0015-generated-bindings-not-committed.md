# 0015. Generated bindings are build output, not committed

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
The Rust core reaches the apps through generated code: UniFFI produces Swift and Kotlin bindings plus native libraries (an XCFramework for iOS, `.so` files for Android), and wasm-bindgen produces the WASM package with TypeScript types for the web ([ADR 0007](0007-rust-core-with-generated-bindings.md)). [monorepo.md](../architecture/monorepo.md) left open whether this output is committed or generated on demand. Development starts locally ([ADR 0017](0017-local-first-development.md)).

## Decision
- Generated bindings and native libraries are **build output**. They are **gitignored** and never committed.
- One command regenerates everything locally: `just bindings`. WASM and Android bindings are built in the Docker `toolbox` container, and iOS bindings on the macOS host ([ADR 0017](0017-local-first-development.md)). It writes to fixed, ignored paths:
  - `core/ffi/generated/swift/` + `core/ffi/generated/Ouril.xcframework` → used by `apps/ios` as a local Swift package
  - `core/ffi/generated/kotlin/` + `core/ffi/generated/jniLibs/` → used by `apps/android`; a Gradle task calls `just bindings-android` before building
  - `core/wasm/pkg/` → used by `apps/web` as a workspace package
- Each app's build checks that the bindings exist and are newer than the core sources, and tells you to run `just bindings` if not.
- CI always generates bindings fresh, so the committed core is the only source of truth.

## Alternatives considered
- **Commit the generated code:** app-only contributors wouldn't need Rust, but it causes noisy diffs, merge conflicts and drift (hand edits, stale output). Large binaries (XCFramework, `.so`) would bloat the repo.
- **Publish prebuilt packages** (SPM/Maven/npm) from CI: clean for consumers, but too much machinery for local-first development with a small team. We can revisit this if app-only contributors appear.

## Consequences
- Bindings can never drift from the core, and no hook is needed to block hand edits of generated files.
- Building an app needs the bindings. For web and Android they come from the `toolbox` container (no host Rust needed). iOS developers also need Xcode on macOS.
- A first build takes longer, because it compiles the core for several targets. Cargo caching keeps later builds fast.

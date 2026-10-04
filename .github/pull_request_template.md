<!--
Thanks for contributing! Keep the PR focused on one concern.
Title: Conventional Commits, e.g. `feat(engine): …`, `fix(ios): …`, `docs: …`.
Delete sections that don't apply.
-->

## Summary

<!-- What does this change and why? One or two sentences. -->

Closes #<!-- issue number -->
ADR: <!-- docs/decisions/NNNN-….md, or "none needed" -->

## Type of change

- [ ] Bug fix
- [ ] Feature
- [ ] Game rules / variant (spec, config or test vectors)
- [ ] Docs only
- [ ] Refactor, tooling or CI (no behaviour change)
- [ ] Breaking change to a public contract (API, realtime protocol, sync format, game record, test-vector format)

## Affected areas

- [ ] `core/` (engine, ai, sync, protocol, ffi, wasm, test-vectors)
- [ ] iOS
- [ ] Android
- [ ] Web
- [ ] Server
- [ ] Docs
- [ ] CI

## Definition of done

See [CONTRIBUTING.md](https://github.com/Alfredo-Moreira/Ouril/blob/main/CONTRIBUTING.md#definition-of-done). Tick what applies; strike through or explain what doesn't.

- [ ] Tests added or updated. For rule changes, test vectors pass in Rust **and** through every binding (Swift, Kotlin, WASM).
- [ ] Rule changes come from a variant spec in [`docs/game/variants/`](https://github.com/Alfredo-Moreira/Ouril/blob/main/docs/game/variants/README.md). "To confirm" behaviour has vectors tagged `to-confirm`.
- [ ] A released variant's behaviour changed → **new variant version** (`id@version`); vectors for released versions untouched ([versioning](https://github.com/Alfredo-Moreira/Ouril/blob/main/docs/architecture/versioning.md)).
- [ ] Docs updated in this PR (including the ADR if needed).
- [ ] User-facing strings added to `shared/i18n/` (English at minimum). Nothing hard-coded.
- [ ] Accessibility checked: screen-reader labels, contrast, tap target size.
- [ ] Works offline where applicable. Guests make no network requests without telemetry consent.
- [ ] No secrets in code or config.
- [ ] Generated bindings regenerated, not hand-edited.

## Screenshots

<!-- For UI changes. Light and dark mode if relevant. -->

| iOS | Android | Web |
|---|---|---|
| | | |

## Testing notes

<!-- How did you test this? Devices, OS versions, browsers, variants (`id@version`). Anything reviewers should try. -->

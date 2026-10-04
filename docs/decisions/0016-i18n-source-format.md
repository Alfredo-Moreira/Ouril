# 0016. Translation source format: ICU messages in JSON, generated per platform

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
UI text must live in `shared/i18n/` and be shared by iOS, Android, web and the server (push text) ([localization](../product/features/localization.md)). Each platform has its own native format: String Catalogs on iOS, `strings.xml` on Android, JSON for i18next on the web. We need one source format that can express plurals ("1 seed", "3 seeds") and be converted reliably into all three.

## Decision
- **Source files:** `shared/i18n/<locale>.json`, one per language: `en.json` (source of truth), then `pt.json` and `kea.json` (Cape Verdean Kriolu).
- **Keys** are flat, dotted and stable: `game.capture.toast`, `settings.telemetry.crash.title`.
- **`en.json`** entries carry a description for translators:
  ```json
  {
    "game.captured": {
      "message": "{count, plural, one {You captured # seed} other {You captured # seeds}}",
      "description": "Toast after a capture; count = seeds captured"
    }
  }
  ```
  Other locales map keys to messages only: `{ "game.captured": "…" }`.
- **Messages use ICU MessageFormat, limited to** simple arguments `{name}`, `plural` (with `#`) and `select`. No nesting deeper than one plural/select. This subset converts cleanly to every platform.
- **Generator:** `tools/i18n-gen` (a small Rust CLI in the workspace), run by `just i18n`. It outputs:
  - iOS: `Localizable.xcstrings` (String Catalog with plural variations)
  - Android: `values[-<locale>]/strings.xml` with `<plurals>` (Kriolu uses the `b+kea` qualifier)
  - Web: per-locale JSON for `i18next` with the ICU plugin
  - Server: a Rust module for push-notification text

  Like the bindings, generated files are build output and gitignored ([ADR 0015](0015-generated-bindings-not-committed.md)).
- **Checks** (generator and CI): valid ICU in the supported subset, the same arguments in every locale, no keys missing from `en.json`. Missing translations fall back to English, and a report lists them.

## Alternatives considered
- **Use one platform's format as the source** (for example `.xcstrings` or Android XML): converting from a platform format to the others is lossy and awkward to edit by hand.
- **Translation service as source of truth** (Crowdin, Weblate): useful later for community translators, and it can sync these JSON files, but we don't want it as a dependency now.
- **Fluent (`.ftl`):** expressive, but has weaker native tooling on iOS and Android than ICU.

## Consequences
- One place to add a string. The generator guarantees all platforms get it.
- The restricted ICU subset means some complex phrasing has to be split into separate keys.
- Kriolu (`kea`) needs checking on each platform's locale support. The generator falls back to Portuguese, then English, where a platform lacks `kea`.

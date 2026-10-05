# Localization

> How Ouril supports multiple languages. **Status:** Draft

## Plan

| Phase | Languages |
|---|---|
| 1 (MVP) | **English** (default). All UI strings externalized from day one. |
| Next, as translators are found | **Portuguese**, **Cape Verdean Kriolu**: each ships once a native speaker has translated and reviewed it, not tied to a roadmap phase |
| Later | French (Côte d'Ivoire, Senegal), Spanish, and others based on demand |

## Approach

- All UI text comes from translation files, never hard-coded in views.
- **One source of truth:** `shared/i18n/<locale>.json` (`en.json` is the source, then `pt.json` and `kea.json`). Keys are flat and dotted (`game.captured`). Messages use a restricted subset of ICU MessageFormat (arguments, `plural`, `select`), and English entries carry a description for translators. Full format in [ADR 0016](../../decisions/0016-i18n-source-format.md).
- **Generated per platform** by `tools/i18n-gen` (`just i18n`): iOS String Catalogs (`.xcstrings`), Android `strings.xml` with `<plurals>` (Kriolu uses the `b+kea` qualifier), web JSON for `i18next` with ICU, and a Rust module for server push text. Generated files are build output, not committed.
  - **As built (interim):** `tools/i18n-gen` doesn't exist yet. The web app runs its own script, `apps/web/scripts/gen-i18n.mjs`, before `dev`, `build`, `test` and `typecheck`. It turns `shared/i18n/*.json` into i18next resources in `apps/web/src/generated/i18n/` (gitignored), and checks ICU syntax, translator descriptions and unknown keys. Only English (`en.json`, 194 keys) exists so far.
- **Checks:** valid ICU, the same arguments in every locale, no unknown keys. Missing translations fall back to English (Kriolu falls back to Portuguese, then English, where a platform lacks `kea`).
- The default language comes from the device or browser locale, falling back to English. Players can override it in settings.
- On the web, adding `shared/i18n/<locale>.json` is enough for the app to offer that language: missing keys fall back to English, and the page description (`app.description`) follows the language. A test fails if a component contains hard-coded text or labels (`apps/web/src/i18n/hardcoded.test.ts`). The install manifest has a single (English) name and description for now.
- Dates and numbers are formatted with each platform's locale APIs.
- Game terms follow the [glossary](../../glossary.md). Each variant may use its own traditional names in each language.

## Kriolu considerations

- **Language code:** `kea` (ISO 639-3, Kabuverdianu).
- Spelling differs between islands (for example Santiago vs São Vicente). We'll use **ALUPEC** (the official Cape Verdean alphabet) as the written standard, and may add island variants later.
- Translations need review by native speakers.

## Open questions

- Who translates and reviews PT and Kriolu?
- Do we need a translation management service (for example Crowdin or Weblate), or are files in the repo enough?

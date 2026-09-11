# tools/

Pipeline scripts. Nothing here is linked into the app.

| Script | Purpose |
| --- | --- |
| `vendor-fixtures.py` | Copy rowplay-studio's golden parity fixtures into `tests/fixtures/` and rewrite `manifest.json`. |
| `convert-locales.mjs` | Regenerate the ID-based Qt `.ts` catalogues in `i18n/` from the web locales in `reference/rowplay/src/lib/locales/` (`--check` verifies the committed files; needs Node ≥ 23.6 for type stripping). |

Later phases add asset vendoring and `balsam` pre-processing (Phase 5), and
the Node venue exporter plus Blender clean-up scripts (Phase 6).

# tools/

Pipeline scripts. Nothing here is linked into the app.

| Script | Purpose |
| --- | --- |
| `vendor-fixtures.py` | Copy rowplay-studio's golden parity fixtures into `tests/fixtures/` and rewrite `manifest.json`. |

Later phases add the locale converter (Phase 4), asset vendoring and `balsam`
pre-processing (Phase 5), and the Node venue exporter plus Blender clean-up
scripts (Phase 6).

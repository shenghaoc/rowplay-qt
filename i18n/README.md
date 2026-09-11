# i18n/

ID-based Qt Linguist (`.ts`) files generated from the rowplay web app's six
locales (en, zh, de, es, fr, ja) — never edit them by hand.

Regenerate (needs the web checkout in `reference/`, see `AGENTS.md`):

```bash
node tools/convert-locales.mjs           # write i18n/rowplay_<lang>.ts
node tools/convert-locales.mjs --check   # CI-style freshness check
```

Each message id is the web's dotted key (`dashboard.title`); the `<source>` is
the English text and any key missing in a locale is filled with the English
value, matching the web's per-key fallback. QML resolves ids through
`Tr.t(id, vars)` (`qsTrId` + `{name}` interpolation, web `interpolate`
parity); the web has no plural rules, so there are no numerus forms.

`crates/rowplay-app/build.rs` runs `lrelease` over these files and bundles
`qml_<lang>.qm` into the app's Qt resources at
`:/qt/qml/RowPlay/i18n/qml_<lang>.qm`, where `QQmlApplicationEngine` picks
them up automatically when `Qt.uiLanguage` changes. The Qt-free parity tests
live in `crates/rowplay-viewmodel/tests/i18n_parity.rs`.

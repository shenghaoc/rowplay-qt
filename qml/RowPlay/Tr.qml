// SPDX-License-Identifier: GPL-3.0-or-later
// ID-based translation helper.
//
// `t(id, vars)` resolves the web app's dotted message ids ("dashboard.title")
// through qsTrId — the .qm bundles in :/qt/qml/RowPlay/i18n/ are generated
// from the web locales with those ids (tools/convert-locales.mjs) — and
// interpolates {name} placeholders exactly like the web's `interpolate`
// (src/lib/i18n.ts): a replaceAll per variable with String(value).
//
// The web has no plural rules, so there are no numerus forms. qsTrId returns
// the id itself when no translation is found (verified on Qt 6.11.2), so
// missing translations stay visible; the empty-string guard is a belt for
// older engines.
pragma Singleton
import QtQuick

QtObject {
    function t(id, vars) {
        var text = qsTrId(id)
        if (text.length === 0) {
            text = id
        }
        if (!vars) {
            return text
        }
        // The web uses replaceAll; Qt's ECMAScript engine does not provide it
        // yet, so split/join gives the identical substitution semantics.
        for (var key in vars) {
            text = text.split("{" + key + "}").join(String(vars[key]))
        }
        return text
    }
}

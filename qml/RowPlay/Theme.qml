// SPDX-License-Identifier: GPL-3.0-or-later
// Design tokens — rowplay-qt's own cross-platform design system (ADR 0013).
//
// One visual system, the same on Linux, macOS and Windows. A thin platform
// layer adapts only what users expect from their OS:
// - every font size and every length derives from the system font
//   (Qt.application.font: SF on macOS, Segoe UI on Windows, the desktop
//   default on Linux) as a ratio of a 13 px design reference, so a larger
//   system text size scales the whole UI instead of clipping it;
// - the accent (selection, focus rings, switch-on, prominent buttons)
//   follows the system accent, with the brand blue as the fallback — the
//   metric colours never follow it;
// - the colour scheme follows Qt.styleHints.colorScheme, and under the OS
//   contrast preference (Qt 6.10 QStyleHints::accessibility()) every colour
//   role comes from the system palette's pairs, as Windows' contrast themes
//   require (window / windowText, highlight / highlightedText, button /
//   buttonText, placeholder and disabled text), with 2 px outlines where two
//   surfaces become the same colour.
//
// The PM5 palette and the Metric Mapping Rule are Studio's DESIGN.md ("The
// Erg Display"). The neutral surface and text ramps are this repository's
// own and pass WCAG AA wherever they are used (docs/source-map.md). Flat by
// default: tonal surfaces, no shadows. All views reference these tokens
// instead of hardcoding colours, sizes or type.
pragma Singleton
import QtQuick

QtObject {
    id: theme

    // MARK: - Platform inputs

    // Follows the system colour scheme; Qt.ColorScheme.Unknown (no portal /
    // platform support) resolves to light, DESIGN.md's default aesthetic.
    // ROWPLAY_FORCE_COLOR_SCHEME (Settings.colorSchemeOverride) pins the
    // scheme for tests and CI screenshots. Under high contrast the system
    // palette decides: its window colour says light or dark, so the scheme
    // always matches the colours it is drawn in.
    readonly property bool dark: {
        if (highContrast) {
            return luminance(hcWindow) < 0.5
        }
        const forced = Settings.colorSchemeOverride
        if (forced === "dark") {
            return true
        }
        if (forced === "light") {
            return false
        }
        return Qt.styleHints.colorScheme === Qt.Dark
    }

    // The OS contrast preference (Windows high-contrast themes, macOS
    // "Increase contrast", the desktop portal's contrast setting on Linux).
    // ROWPLAY_FORCE_CONTRAST=high|normal (Settings.contrastOverride) pins it.
    readonly property bool highContrast: {
        const forced = Settings.contrastOverride
        if (forced === "high") {
            return true
        }
        if (forced === "normal") {
            return false
        }
        return Qt.styleHints.accessibility.contrastPreference === Qt.HighContrast
    }

    /// The system font's size in pixels — the base of the whole scale.
    readonly property real basePx: {
        const font = Qt.application.font
        if (font.pixelSize > 0) {
            return font.pixelSize
        }
        const screens = Qt.application.screens
        const pxPerMm = screens.length > 0 ? screens[0].logicalPixelDensity : 96 / 25.4
        return Math.max(9, font.pointSize * pxPerMm * 25.4 / 72)
    }
    /// 1.0 at the 13 px design reference.
    readonly property real scale: basePx / 13

    /// A design-reference length, scaled with the system font and rounded
    /// to whole pixels.
    function px(reference) {
        return Math.max(1, Math.round(reference * scale))
    }

    /// Whether interface motion is reduced (the app's reduce-motion
    /// preference stills the shell's own animations too).
    readonly property bool reduceMotion: Settings.reduceReplayMotion
    /// Standard duration of a control transition (0 under reduce motion).
    readonly property int motionDuration: reduceMotion ? 0 : 140

    // MARK: - Contrast helpers (WCAG relative luminance)

    function luminance(value) {
        const c = Qt.lighter(value, 1.0)
        function channel(v) {
            return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }

    function contrastRatio(a, b) {
        const la = luminance(a)
        const lb = luminance(b)
        return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05)
    }

    /// White or near-black, whichever contrasts more with `fill` (a label
    /// on a solid badge or on the accent); under high contrast the system
    /// palette's window or window-text colour.
    function textOn(fill) {
        const light = highContrast ? (dark ? hcWindowText : hcWindow) : "#ffffff"
        const deep = highContrast ? (dark ? hcWindow : hcWindowText) : "#0e1014"
        return contrastRatio(fill, light) >= contrastRatio(fill, deep) ? light : deep
    }

    /// `foreground` composited over the opaque `background`: what the screen
    /// shows. macOS reports its text roles with alpha (window text is black
    /// at 85 %, placeholders at 50 %), so contrast is measured on this.
    function over(foreground, background) {
        const f = Qt.lighter(foreground, 1.0)
        const b = Qt.lighter(background, 1.0)
        const a = f.a
        return Qt.rgba(f.r * a + b.r * (1 - a), f.g * a + b.g * (1 - a),
                       f.b * a + b.b * (1 - a), 1)
    }

    /// A colour of our own palette (a metric or a status colour) kept under
    /// high contrast only where it reaches 4.5:1 on `surface` (the window by
    /// default); otherwise the window text. The label beside a metric names
    /// it, so no meaning is lost.
    function hcFit(colour, surface) {
        if (!highContrast) {
            return colour
        }
        const base = surface === undefined ? hcWindow : surface
        return contrastRatio(colour, base) >= 4.5 ? colour : hcWindowText
    }

    // MARK: - System palette (high contrast)
    //
    // Under the OS contrast preference, surfaces, text, selection, controls
    // and lines take the system palette's colour pairs (Windows maps a
    // contrast theme's Window, WindowText, Highlight, HighlightText,
    // ButtonFace, ButtonText and GrayText there), composited to opaque
    // colours. What Qt 6.11 reports per OS is in docs/qt-bridges-notes.md.

    readonly property SystemPalette systemPaletteDisabled: SystemPalette {
        colorGroup: SystemPalette.Disabled
    }
    readonly property color hcWindow: over(systemPalette.window, "#808080")
    readonly property color hcWindowText: over(systemPalette.windowText, hcWindow)
    readonly property color hcButton: over(systemPalette.button, hcWindow)
    readonly property color hcButtonText: over(systemPalette.buttonText, hcButton)
    readonly property color hcHighlight: over(systemPalette.highlight, hcWindow)
    readonly property color hcHighlightedText: over(systemPalette.highlightedText, hcHighlight)
    /// Disabled text only (GrayText on Windows).
    readonly property color hcGrayText: over(systemPaletteDisabled.windowText, hcWindow)
    /// Placeholders, where they reach 4.5:1 in a field; the window text
    /// otherwise.
    readonly property color hcPlaceholder: {
        const placeholder = over(systemPalette.placeholderText, hcButton)
        return contrastRatio(placeholder, hcButton) >= 4.5 ? placeholder : hcButtonText
    }

    // MARK: - Accent (system accent, brand blue fallback)

    readonly property SystemPalette systemPalette: SystemPalette {
        colorGroup: SystemPalette.Active
    }
    /// macOS and Windows report the user's accent. Where the platform
    /// supplies none (no Linux theme sets one in Qt 6.11, nor does the
    /// offscreen platform), Qt reports its Fusion default #308cc6, which
    /// means "no accent" here (docs/qt-bridges-notes.md).
    readonly property bool systemAccentAvailable: systemPalette.accent.a > 0
                                                  && !Qt.colorEqual(systemPalette.accent, "#308cc6")
    /// Monitor Blue (DESIGN.md: primary actions and the accent colour role).
    readonly property color brandBlue: dark ? "#0A84FF" : "#0066CC"
    /// Selection, switch-on and prominent buttons; the system highlight
    /// under high contrast.
    readonly property color accentColor: highContrast ? hcHighlight
                                         : (systemAccentAvailable ? systemPalette.accent
                                                                  : brandBlue)
    /// Text and glyphs on an accent fill: white or near-black, whichever
    /// contrasts more with the accent in use (AA for the brand blues); the
    /// system's highlighted text under high contrast.
    readonly property color onAccent: highContrast ? hcHighlightedText : textOn(accentColor)

    // MARK: - Colour palette (DESIGN.md "The PM5 Palette"; under high
    // contrast each colour stays only where it reaches 4.5:1, see hcFit)

    /// Primary brand blue — distance, key emphasis.
    readonly property color primaryBlue: hcFit(dark ? "#0A84FF" : "#0066CC")
    /// Warm comparison orange — watts, splits, secondary emphasis.
    readonly property color comparisonOrange: hcFit(dark ? "#FF9F0A" : "#9A5700")
    /// Energetic green — positive deltas, success states, cadence highlights.
    readonly property color energeticGreen: hcFit(dark ? "#30D158" : "#137333")
    /// Alert red — negative deltas, heart rate, finish markers.
    readonly property color alertRed: hcFit(dark ? "#FF453A" : "#B3261E")
    /// Destructive button labels: the alert red, lifted in dark mode, where
    /// the PM5 red measures 4.46:1 on the control fill (AA needs 4.5).
    readonly property color destructiveText: hcFit(dark ? "#FF6B61" : "#B3261E", hcButton)
    /// Soft purple — elevation, descent, cadence accents.
    readonly property color softPurple: hcFit(dark ? "#BF5AF2" : "#7B2CBF")
    /// Warm yellow — caution states, active indicators.
    readonly property color warmYellow: hcFit(dark ? "#FFD60A" : "#7A5A00")

    // MARK: - Semantic metric colours (the Metric Mapping Rule: one colour
    // per metric domain, never cross-assigned, never the accent)

    readonly property color metricDistance: primaryBlue
    readonly property color metricDuration: hcFit(dark ? "#64D2FF" : "#007A99")
    /// Slightly lighter blue than distance in dark mode, so pace and distance
    /// stay distinguishable (DesignTokens MetricColor.pace).
    readonly property color metricPace: hcFit(dark ? "#409CFF" : "#0066CC")
    readonly property color metricSpeed: comparisonOrange
    readonly property color metricWatts: comparisonOrange
    readonly property color metricHeartRate: alertRed
    readonly property color metricCadence: softPurple
    readonly property color metricSplit: comparisonOrange

    /// Maps a `ColorRole` index (rowplay-viewmodel) to its palette colour
    /// (the Metric Mapping Rule: one colour per metric domain).
    function metricColor(role) {
        switch (role) {
        case 1: return metricDistance
        case 2: return metricDuration
        case 3: return metricPace
        case 4: return metricWatts
        case 5: return metricHeartRate
        case 6: return metricCadence
        default: return textPrimary
        }
    }

    /// Green/red for positive/negative deltas with a dead-zone threshold
    /// (DesignTokens.deltaColor). `delta` may be null/undefined; `higherIsBetter`
    /// flips the sign convention (watts/distance vs pace). Colour is never
    /// the only carrier: a delta's text keeps its sign or word.
    function deltaColor(delta, threshold, higherIsBetter) {
        if (threshold === undefined) threshold = 0.5
        if (higherIsBetter === undefined) higherIsBetter = false
        if (delta === null || delta === undefined || !isFinite(delta)
                || Math.abs(delta) < threshold) {
            return textSecondary
        }
        const positive = higherIsBetter ? delta > 0 : delta < 0
        return positive ? energeticGreen : alertRed
    }

    // MARK: - Surfaces (our neutral ramp; tonal layering, no shadows. Under
    // high contrast every surface is the system window colour and controls
    // the button colour; the outlines below keep them apart.)

    /// The content canvas and the toolbar above it.
    readonly property color windowBackground: highContrast ? hcWindow
                                              : (dark ? "#111317" : "#FFFFFF")
    readonly property color toolbarBackground: windowBackground
    /// The sidebar column.
    readonly property color sidebarBackground: highContrast ? hcWindow
                                               : (dark ? "#16191D" : "#F4F5F7")
    /// Grouped surfaces: cards, grouped settings rows, chart panels. The
    /// light value keeps the PM5 duration colour at AA on it (4.53:1; it
    /// measured 4.496:1 on #F3F4F6).
    readonly property color groupBackground: highContrast ? hcWindow
                                             : (dark ? "#191C21" : "#F4F5F7")
    readonly property color panelBackground: groupBackground
    readonly property color cardBackground: groupBackground
    /// A selected card (tonal accent wash; the highlight in high contrast).
    readonly property color activeCardBackground: highContrast
                                                  ? hcHighlight
                                                  : Qt.rgba(accentColor.r, accentColor.g,
                                                            accentColor.b, 0.12)
    /// Text fields, push buttons, pop-up buttons.
    readonly property color controlBackground: highContrast ? hcButton
                                               : (dark ? "#22262C" : "#FFFFFF")
    /// Menus, pop-up lists, tooltips, dialogs.
    readonly property color popupBackground: highContrast ? hcWindow
                                             : (dark ? "#1C1F24" : "#FFFFFF")
    /// Floating controls over the replay scene: opaque, on the grouped
    /// surface. Translucent (0.88–0.90 alpha), the metric colours and the
    /// tertiary text fell below AA over dark parts of the scene.
    readonly property color overlayBackground: groupBackground

    // MARK: - Text (WCAG AA on every surface above; see docs/source-map.md)

    readonly property color textPrimary: highContrast ? hcWindowText
                                         : (dark ? "#ECEEF2" : "#15181D")
    readonly property color textSecondary: highContrast ? hcWindowText
                                           : (dark ? "#A9B0BA" : "#535A65")
    /// Placeholders and decoration only; still ≥ 4.5:1. Under high
    /// contrast the system's placeholder colour where it reaches 4.5:1.
    readonly property color textTertiary: highContrast ? hcPlaceholder
                                          : (dark ? "#8E959F" : "#666D78")
    /// Disabled labels (exempt from contrast requirements; still legible);
    /// the system's disabled text under high contrast.
    readonly property color textDisabled: highContrast ? hcGrayText
                                          : (dark ? "#646B75" : "#A1A8B2")

    // MARK: - Lines, washes and control parts

    readonly property color separator: highContrast ? hcWindowText
                                       : (dark ? "#2A2E35" : "#DDE1E6")
    /// Control outlines (≥ 3:1 against the surfaces they sit on).
    readonly property color controlBorder: highContrast ? hcButtonText
                                           : (dark ? "#6B737E" : "#838B96")
    readonly property color hoverFill: highContrast ? Qt.alpha(hcWindowText, 0.12)
                                       : (dark ? Qt.rgba(1, 1, 1, 0.06)
                                               : Qt.rgba(0, 0, 0, 0.045))
    readonly property color pressedFill: highContrast ? Qt.alpha(hcWindowText, 0.24)
                                         : (dark ? Qt.rgba(1, 1, 1, 0.11)
                                                 : Qt.rgba(0, 0, 0, 0.09))
    /// Selection with keyboard focus: the accent, its text onAccent.
    readonly property color selectionFill: accentColor
    readonly property color selectionText: onAccent
    /// Selection without focus: a neutral wash, primary text. Under high
    /// contrast the window fill with a 2 px outline in the highlight
    /// (selectionOutline), so it still differs from the focused selection.
    readonly property color selectionFillInactive: highContrast
                                                   ? hcWindow
                                                   : (dark ? Qt.rgba(1, 1, 1, 0.10)
                                                           : Qt.rgba(0, 0, 0, 0.075))
    readonly property color selectionOutline: highContrast ? hcHighlight : "transparent"
    readonly property color segmentTrack: highContrast ? hcButton
                                          : (dark ? "#22262C" : "#E8EAEE")
    /// The selected segment's thumb and label: the highlight pair under
    /// high contrast.
    readonly property color segmentThumb: highContrast ? hcHighlight
                                          : (dark ? "#3A3F47" : "#FFFFFF")
    readonly property color segmentThumbText: highContrast ? hcHighlightedText : textPrimary
    readonly property color switchTrackOff: highContrast ? hcButton
                                            : (dark ? "#3A3F47" : "#D4D8DE")
    /// The knob on an off switch and on the slider; switchKnobOn on an on
    /// switch's accent track.
    readonly property color switchKnob: highContrast ? hcButtonText : "#FFFFFF"
    readonly property color switchKnobOn: highContrast ? hcHighlightedText : "#FFFFFF"
    /// Keyboard focus ring, two-tone (FocusRing.qml): the outer band in the
    /// accent, or the primary text colour when an unusual system accent
    /// would fall below 3:1 against the window (always under high contrast),
    /// and the inner band in the window colour, so one of the two contrasts
    /// with whatever the ring surrounds or crosses.
    readonly property color focusRing: highContrast
                                       || contrastRatio(accentColor, windowBackground) < 3
                                       ? textPrimary : accentColor
    readonly property color focusRingInner: windowBackground
    readonly property int focusRingWidth: 2
    readonly property int focusRingInnerWidth: 1
    /// How far the ring reaches outside its control.
    readonly property int focusRingExtent: focusRingWidth + focusRingInnerWidth
    readonly property color chartGrid: highContrast ? Qt.alpha(hcWindowText, 0.4)
                                       : (dark ? "#23272D" : "#ECEEF1")
    readonly property color chartAxis: highContrast ? hcWindowText
                                       : (dark ? "#4A5059" : "#B8BEC7")

    // MARK: - Outlines under high contrast
    //
    // There every surface is the window colour, so the edges that tones
    // drew become 2 px outlines in the separator colour.

    /// Cards and panels (no outline on the tonal ramps).
    readonly property int cardBorderWidth: highContrast ? 2 : 0
    /// Outlines drawn in both modes: grouped forms, popups, the replay HUD.
    readonly property int outlineWidth: highContrast ? 2 : hairline
    /// Structural rules: the sidebar's edge and the toolbar's.
    readonly property int ruleWidth: highContrast ? 2 : hairline

    // MARK: - Replay materials (Phase 5a)
    //
    // The V3/V4 packs ship one neutral placeholder material; product colour
    // lives here so the 3D scene shares the shell's light/dark palettes.
    // `rowplay_viewmodel::replay::materials` maps every replayMaterialRole to
    // one of these tokens (or to the venue palette for lane paint) — QML must
    // not hardcode material hex values.

    /// Athlete skin (V4 vertex-colour skin regions).
    readonly property color replaySkin: dark ? "#c98d68" : "#d99a72"
    /// Athlete top / primary fabric (V3 `athlete-fabric`).
    readonly property color replayFabric: dark ? "#2f6fb2" : "#2a62a0"
    /// Athlete hair.
    readonly property color replayHair: dark ? "#3a2b22" : "#4a372c"
    /// Athlete footwear.
    readonly property color replayFootwear: dark ? "#d8d8dc" : "#eceef0"
    /// Athlete shorts (V4 lower body).
    readonly property color replayShorts: dark ? "#23262b" : "#2c3036"
    /// Athlete trim / accents.
    readonly property color replayTrim: dark ? "#ffd60a" : "#b58900"
    /// Athlete eyes.
    readonly property color replayEye: dark ? "#e8ecef" : "#f4f7f9"
    /// Athlete face detail (brows, lips).
    readonly property color replayFaceDetail: dark ? "#8a5a44" : "#96604a"
    /// Dark equipment surfaces (rails, frames, flywheel housings).
    readonly property color replayEquipmentDark: dark ? "#1d2126" : "#23272c"
    /// Light equipment surfaces (seats, hulls, shells).
    readonly property color replayEquipmentLight: dark ? "#c9ced4" : "#e4e8ec"
    /// Metal equipment parts (poles, riggers, chains).
    readonly property color replayEquipmentMetal: dark ? "#9aa3ab" : "#b9c1c8"
    /// Rubber (feet, grips' base, tyres).
    readonly property color replayEquipmentRubber: dark ? "#141618" : "#1a1d20"
    /// Handle grips.
    readonly property color replayEquipmentGrip: dark ? "#2b2e33" : "#33373c"
    /// Equipment trim accents.
    readonly property color replayEquipmentTrim: dark ? "#ff9f0a" : "#9a5700"

    // MARK: - Spacing scale (8-point soft grid, scaled with the system font)

    readonly property int spacingXxSmall: px(2)    // hairline gaps
    readonly property int spacingXSmall: px(4)     // tight gaps within components
    readonly property int spacingSmall: px(6)      // compact gaps
    readonly property int spacingMedium: px(8)     // default inner padding
    readonly property int spacingLarge: px(12)     // standard component spacing
    readonly property int spacingXLarge: px(16)    // section-level spacing
    readonly property int spacingXxLarge: px(20)   // generous section gaps
    readonly property int spacingXxxLarge: px(24)  // major section separation

    // MARK: - Corner radii

    readonly property int radiusSmall: px(6)   // controls, badges, tags
    readonly property int radiusMedium: px(8)  // cards, panels
    readonly property int radiusLarge: px(12)  // grouped forms, overlays
    readonly property int radiusXLarge: px(16) // hero cards

    // MARK: - Control metrics (about 32 px controls at the 13 px reference)

    readonly property int controlHeight: px(32)
    readonly property int toolbarHeight: px(52)
    readonly property int sidebarRowHeight: px(48)
    readonly property int formRowHeight: px(44)
    readonly property int iconSize: px(16)
    readonly property int hairline: 1

    /// Columns for a grid of `count` equal-width cells at least `minWidth`
    /// wide: as many as fit, then spread over the rows so they stay balanced
    /// (four tiles make one row of four or two rows of two, never 3 + 1).
    /// Layout arithmetic only.
    function balancedColumns(count, width, minWidth, gap) {
        if (!(count > 0)) {
            return 1
        }
        var fit = Math.floor((width + gap) / (minWidth + gap))
        fit = Math.min(count, Math.max(2, fit))
        var rows = Math.ceil(count / fit)
        return Math.ceil(count / rows)
    }

    // MARK: - Chart sizing

    readonly property int chartHeight: px(220)        // dashboard-level charts
    readonly property int chartStrokeHeight: px(150)  // stroke-level charts

    // MARK: - Typography (DESIGN.md hierarchy on the system font, sized as
    // ratios of the 13 px reference; tabular figures through the OpenType
    // `tnum` feature, Qt 6.11 has no Font.TabularNumbers)
    // The One Hero Rule: heroMetric appears at most once per card.

    function fontPx(reference) {
        return Math.max(8, Math.round(reference * scale))
    }

    /// Page title — main view headings (26 px semibold at the reference).
    readonly property font pageTitle: ({ pixelSize: fontPx(26), weight: Font.DemiBold })
    /// Large hero metric — primary values in summary cards (28 px bold).
    readonly property font heroMetric: ({ pixelSize: fontPx(28), weight: Font.Bold,
                                          features: { "tnum": 1 } })
    /// Section headline — panel titles (15 px semibold).
    readonly property font sectionHeadline: ({ pixelSize: fontPx(15), weight: Font.DemiBold })
    /// Grouped-form section title (13 px bold).
    readonly property font groupTitle: ({ pixelSize: fontPx(13), weight: Font.Bold })
    /// Metric value — data values in badges and cards (13 px semibold).
    readonly property font metricValue: ({ pixelSize: fontPx(13), weight: Font.DemiBold,
                                           features: { "tnum": 1 } })
    /// Card metric — the value on a personal-best card (16 px semibold).
    readonly property font cardMetric: ({ pixelSize: fontPx(16), weight: Font.DemiBold,
                                          features: { "tnum": 1 } })
    /// Strip metric — inline values in detail/replay strips (20 px semibold).
    readonly property font stripMetric: ({ pixelSize: fontPx(20), weight: Font.DemiBold,
                                           features: { "tnum": 1 } })
    /// Metric label — labels beneath values (11 px medium).
    readonly property font metricLabel: ({ pixelSize: fontPx(11), weight: Font.Medium })
    /// Compact label — dense UI labels (10 px medium).
    readonly property font compactLabel: ({ pixelSize: fontPx(10), weight: Font.Medium })
    /// Sidebar day headers (11 px bold, sentence case).
    readonly property font sidebarSection: ({ pixelSize: fontPx(11), weight: Font.Bold })
    /// Body text (13 px regular at the reference: the system font size).
    readonly property font body: ({ pixelSize: fontPx(13), weight: Font.Normal })
    /// Emphasised body — list titles, control labels (13 px medium).
    readonly property font bodyEmphasized: ({ pixelSize: fontPx(13), weight: Font.Medium })
    /// Tabular body — numbers in lists and tables (13 px semibold, tnum).
    readonly property font tabularBody: ({ pixelSize: fontPx(13), weight: Font.DemiBold,
                                           features: { "tnum": 1 } })
    /// Subheadline metadata (12 px).
    readonly property font subheadline: ({ pixelSize: fontPx(12), weight: Font.Normal })
    /// Chart axis labels (10 px, tnum).
    readonly property font chartLabel: ({ pixelSize: fontPx(10), weight: Font.Normal,
                                          features: { "tnum": 1 } })
}

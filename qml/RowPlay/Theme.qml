// SPDX-License-Identifier: GPL-3.0-or-later
// Design tokens (ADR 0013, ADR 0015).
//
// Standard controls are each platform's own Qt Quick Controls style; these
// tokens serve our own content (charts, metric colours, tiles, the replay
// and its HUD). Relative to the system:
// - surfaces, text and lines derive from the system palette (the window
//   and window-text colours and tonal steps between them), fitted to WCAG
//   AA for text and 3:1 for non-text marks on the surfaces they are drawn
//   on; `dark` follows the palette in use;
// - every font size and every length derives from the system font
//   (Qt.application.font) as a ratio of a 13 px design reference, so a
//   larger system text size scales the whole UI instead of clipping it;
// - the accent (selection, focus rings) follows the system accent, with the
//   brand blue as the fallback; the metric colours never follow it;
// - under the OS contrast preference (Qt 6.10 QStyleHints::accessibility())
//   every colour role comes from the system palette's pairs, as Windows'
//   contrast themes require, with 2 px outlines where two surfaces become
//   the same colour.
//
// The PM5 palette and the Metric Mapping Rule are Studio's DESIGN.md ("The
// Erg Display"); they stay the product's, fitted to 4.5:1 on the system's
// surfaces. Flat: tonal surfaces, no shadows.
pragma Singleton
import QtQuick

QtObject {
    id: theme

    // MARK: - Platform inputs

    // The palette in use decides: its window colour says light or dark, so
    // the tokens always match the colours the native controls draw with.
    // ROWPLAY_FORCE_COLOR_SCHEME asks Qt for a scheme (Main.qml); the macOS
    // and Windows palettes follow the request, and a platform that cannot
    // switch keeps its own.
    readonly property bool dark: luminance(sysWindow) < 0.5

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

    // MARK: - Motion (round 3's 2e: M3's duration and easing tokens)

    /// Whether interface motion is reduced: the app's reduce-motion
    /// preference (Qt 6.11 surfaces no OS one). Every duration below is 0
    /// under it, so all of our own motion is instant; the controls the
    /// style draws animate as the style does.
    readonly property bool reduceMotion: Settings.reduceReplayMotion
    /// A small element's change of state: the scrubber's knob appearing.
    readonly property int durationShort: reduceMotion ? 0 : 150
    /// A panel appearing or leaving: the replay HUD's fade, the drawer.
    readonly property int durationMedium: reduceMotion ? 0 : 250
    /// A change of the whole view. No animation of ours is one today.
    readonly property int durationLong: reduceMotion ? 0 : 400
    /// M3's standard curve, for an element that stays on screen or leaves
    /// it, and its emphasized-decelerate curve, for one that enters: the
    /// control points of an `Easing.BezierSpline`.
    readonly property var easingStandard: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
    readonly property var easingEmphasized: [0.05, 0.7, 0.1, 1.0, 1.0, 1.0]

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

    /// `a` blended toward `b` by `t` (0..1), opaque.
    function mix(a, b, t) {
        const x = Qt.lighter(a, 1.0)
        const y = Qt.lighter(b, 1.0)
        return Qt.rgba(x.r + (y.r - x.r) * t, x.g + (y.g - x.g) * t,
                       x.b + (y.b - x.b) * t, 1)
    }

    /// The lowest contrast `colour` has on any of `surfaces`.
    function worstContrast(colour, surfaces) {
        var worst = Infinity
        for (var i = 0; i < surfaces.length; ++i) {
            worst = Math.min(worst, contrastRatio(colour, surfaces[i]))
        }
        return worst
    }

    /// `colour` moved toward the system text colour only as far as it takes
    /// to reach `target` on every one of `surfaces`: the colour itself where
    /// it passes, the least change where it does not (the PM5 blue on the
    /// macOS dark window's cards measures 4.1:1).
    function fitContrast(colour, surfaces, target) {
        for (var step = 0; step <= 50; ++step) {
            const candidate = mix(colour, sysText, step / 50)
            if (worstContrast(candidate, surfaces) >= target) {
                return candidate
            }
        }
        return sysText
    }

    /// A colour of our own palette (a metric or a status colour) on the
    /// system's surfaces: fitted to 4.5:1 on `surface` (the text surfaces
    /// by default), and under high contrast kept only where it reaches 4.5:1
    /// on its own (hcFit, with its `fallback`). The label beside a metric
    /// names it, so no meaning is lost.
    function paletteColour(colour, surface, fallback) {
        if (highContrast) {
            return hcFit(colour, surface, fallback)
        }
        return fitContrast(colour, surface === undefined ? textSurfaces : [surface], 4.5)
    }

    /// Under high contrast, a colour of our own palette kept only where it
    /// reaches 4.5:1 on `surface` (the window by default); otherwise that surface's text colour (`fallback`, the
    /// window text by default). The label beside a metric names it, so no
    /// meaning is lost.
    function hcFit(colour, surface, fallback) {
        if (!highContrast) {
            return colour
        }
        const base = surface === undefined ? hcWindow : surface
        const text = fallback === undefined ? hcWindowText : fallback
        return contrastRatio(colour, base) >= 4.5 ? colour : text
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
    /// The system's window colour and its text on it, opaque: the base of
    /// every surface, text and line token below (ADR 0015). macOS reports
    /// its text roles with alpha, so they are composited here.
    readonly property color sysWindow: over(systemPalette.window, "#808080")
    readonly property color sysText: over(systemPalette.windowText, sysWindow)
    readonly property color hcWindow: over(systemPalette.window, "#808080")
    readonly property color hcWindowText: over(systemPalette.windowText, hcWindow)
    readonly property color hcButton: over(systemPalette.button, hcWindow)
    readonly property color hcButtonText: over(systemPalette.buttonText, hcButton)
    readonly property color hcHighlight: over(systemPalette.highlight, hcWindow)
    /// Disabled text only (GrayText on Windows).
    readonly property color hcGrayText: over(systemPaletteDisabled.windowText, hcWindow)

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

    // MARK: - Colour palette (DESIGN.md "The PM5 Palette"; under high
    // contrast each colour stays only where it reaches 4.5:1, see hcFit)

    /// Primary brand blue — distance, key emphasis.
    readonly property color primaryBlue: paletteColour(dark ? "#0A84FF" : "#0066CC")
    /// Warm comparison orange — watts, splits, secondary emphasis.
    readonly property color comparisonOrange: paletteColour(dark ? "#FF9F0A" : "#9A5700")
    /// Energetic green — positive deltas, success states, cadence highlights.
    readonly property color energeticGreen: paletteColour(dark ? "#30D158" : "#137333")
    /// Alert red — negative deltas, heart rate, finish markers.
    readonly property color alertRed: paletteColour(dark ? "#FF453A" : "#B3261E")
    /// Soft purple — elevation, descent, cadence accents.
    readonly property color softPurple: paletteColour(dark ? "#BF5AF2" : "#7B2CBF")

    // MARK: - Semantic metric colours (the Metric Mapping Rule: one colour
    // per metric domain, never cross-assigned, never the accent)

    readonly property color metricDistance: primaryBlue
    readonly property color metricDuration: paletteColour(dark ? "#64D2FF" : "#007A99")
    /// Slightly lighter blue than distance in dark mode, so pace and distance
    /// stay distinguishable (DesignTokens MetricColor.pace).
    readonly property color metricPace: paletteColour(dark ? "#409CFF" : "#0066CC")
    readonly property color metricWatts: comparisonOrange
    readonly property color metricHeartRate: alertRed
    readonly property color metricCadence: softPurple

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

    // MARK: - Surfaces (the system palette, ADR 0015: the window colour and
    // tonal steps toward its text; no shadows. Under high contrast every
    // surface is the system window colour and controls the button colour;
    // the outlines below keep them apart.)

    /// The content canvas and the toolbar above it.
    readonly property color windowBackground: highContrast ? hcWindow : sysWindow
    /// The sidebar column.
    readonly property color sidebarBackground: highContrast ? hcWindow
                                               : mix(sysWindow, sysText, 0.04)
    /// Grouped surfaces: cards, grouped settings rows, chart panels.
    readonly property color groupBackground: highContrast ? hcWindow
                                             : mix(sysWindow, sysText, 0.05)
    /// The surfaces text is drawn on, for the contrast fitting.
    readonly property var textSurfaces: [windowBackground, sidebarBackground, groupBackground]
    readonly property color panelBackground: groupBackground
    readonly property color cardBackground: groupBackground
    /// Text fields, push buttons, pop-up buttons: the system's base colour.
    readonly property color controlBackground: highContrast ? hcButton
                                               : over(systemPalette.base, sysWindow)
    /// Floating controls over the replay scene: opaque, on the grouped
    /// surface. Translucent (0.88–0.90 alpha), the metric colours and the
    /// tertiary text fell below AA over dark parts of the scene.
    readonly property color overlayBackground: groupBackground

    // MARK: - Text (the system's text colour, and steps toward the window
    // fitted to WCAG AA on every text surface above)

    readonly property color textPrimary: highContrast ? hcWindowText : sysText
    readonly property color textSecondary: highContrast ? hcWindowText
                                           : fitContrast(mix(sysWindow, sysText, 0.62),
                                                         textSurfaces, 5.5)
    /// Text and glyphs on a control's own fill (`controlBackground`,
    /// `segmentTrack`): the primary text, or under high contrast the
    /// system's button text, which a contrast theme may set apart from its
    /// window text.
    readonly property color controlText: highContrast ? hcButtonText : textPrimary
    /// Disabled labels (exempt from contrast requirements): the system's
    /// disabled text.
    readonly property color textDisabled: highContrast ? hcGrayText
                                          : over(systemPaletteDisabled.windowText, sysWindow)

    // MARK: - Lines, washes and control parts

    readonly property color separator: highContrast ? hcWindowText
                                       : mix(sysWindow, sysText, 0.12)
    /// Control outlines (≥ 3:1 against the surfaces they sit on).
    readonly property color controlBorder: highContrast ? hcButtonText
                                           : fitContrast(mix(sysWindow, sysText, 0.3),
                                                         [windowBackground, groupBackground,
                                                          controlBackground], 3)
    /// A selected sidebar row while its list lacks focus; the active row
    /// uses the style's own highlight and text roles.
    readonly property color selectionFillInactive: highContrast
                                                   ? hcWindow : Qt.alpha(sysText, 0.09)
    readonly property color selectionOutline: highContrast ? hcHighlight : "transparent"
    readonly property color segmentTrack: highContrast ? hcButton
                                          : mix(sysWindow, sysText, 0.08)
    /// The replay scrubber's knob (AppSlider).
    readonly property color switchKnob: highContrast ? hcButtonText : "#FFFFFF"
    /// Keyboard focus ring on our own focusables, two-tone (FocusRing.qml):
    /// the outer band in the accent, moved toward the text colour only as
    /// far as it takes to reach 3:1 on every surface (macOS's blue measures
    /// 2.9:1 on its dark grouped surface), the primary text colour under
    /// high contrast; and the inner band in the window colour, so one of the
    /// two contrasts with whatever the ring surrounds or crosses.
    readonly property color focusRing: highContrast ? textPrimary
                                       : fitContrast(accentColor, textSurfaces, 3)
    readonly property color focusRingInner: windowBackground
    readonly property int focusRingWidth: 2
    readonly property int focusRingInnerWidth: 1
    /// How far the ring reaches outside its control.
    readonly property int focusRingExtent: focusRingWidth + focusRingInnerWidth
    readonly property color chartGrid: highContrast ? Qt.alpha(hcWindowText, 0.4)
                                       : mix(groupBackground, sysText, 0.07)
    /// Axis lines, 3:1 on the chart panel (non-text contrast).
    readonly property color chartAxis: highContrast ? hcWindowText
                                       : fitContrast(mix(groupBackground, sysText, 0.3),
                                                     [groupBackground], 3)

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
    // not hardcode material hex values. The one exception is the rowing
    // scene's palette (ADR 0016) in `Replay/RowingStyle.qml`: the shell and
    // oars, water, fog, key light and venue tints. It colours only the 3D
    // scene, never text or controls.

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

    // MARK: - Control metrics (about 32 px controls at the 13 px reference)

    readonly property int controlHeight: px(32)
    readonly property int toolbarHeight: px(52)
    readonly property int iconSize: px(16)
    readonly property int hairline: 1

    /// Columns for a grid of `count` equal-width cells at least `minWidth`
    /// wide: as many as fit, then spread over the rows so they stay balanced
    /// (four tiles make one row of four or two rows of two, never 3 + 1).
    /// One column where not even two fit (a wide sidebar beside a narrow
    /// window). Layout arithmetic only.
    function balancedColumns(count, width, minWidth, gap) {
        if (!(count > 0)) {
            return 1
        }
        var fit = Math.floor((width + gap) / (minWidth + gap))
        fit = Math.min(count, Math.max(1, fit))
        var rows = Math.ceil(count / fit)
        return Math.ceil(count / rows)
    }

    // MARK: - Width classes
    //
    // M3's window size classes (compact < 600, medium 600–839, expanded
    // 840–1199, large 1200–1599, extra-large ≥ 1600), scaled with the
    // system font like every other length, so larger text reaches the
    // narrower layouts in a wider window.
    // - Expanded and up: the full layout, the sidebar beside the content.
    // - Medium: the sidebar becomes a drawer over the content, and the
    //   toolbar's sport filter a pop-up button.
    // - Compact: the content is one column, and the replay HUD stacks its
    //   rows.
    // - Large and extra-large (round 3's 2d): the content's own layouts
    //   widen. The detail opens a supporting pane and the dashboard its
    //   feed, each where its own column is wide enough, since the sidebar
    //   takes part of the window. Text and forms stay at a readable width.

    readonly property int breakpointMedium: px(600)
    readonly property int breakpointExpanded: px(840)
    readonly property int breakpointLarge: px(1200)
    readonly property int breakpointExtraLarge: px(1600)
    readonly property int widthCompact: 0
    readonly property int widthMedium: 1
    readonly property int widthExpanded: 2
    readonly property int widthLarge: 3
    readonly property int widthExtraLarge: 4

    /// The width class of a window `width` pixels wide.
    function widthClass(width) {
        return width < breakpointMedium ? widthCompact
             : width < breakpointExpanded ? widthMedium
             : width < breakpointLarge ? widthExpanded
             : width < breakpointExtraLarge ? widthLarge : widthExtraLarge
    }

    /// The widest a column of running text or a form grows: some 90
    /// characters of body text, and the settings page's width.
    readonly property int readableWidth: px(640)
    /// The widest the dashboard's and the detail's content grows; wider,
    /// it stays centred (extra-large windows).
    readonly property int contentMaxWidth: px(1440)
    /// The least width of each of two side-by-side panes on the detail
    /// and of each dashboard chart side by side, and the widest a tile or
    /// a personal-best card grows.
    readonly property int paneMinWidth: px(460)
    readonly property int tileMaxWidth: px(320)

    // MARK: - Chart sizing

    readonly property int chartHeight: px(220)        // dashboard-level charts
    readonly property int chartStrokeHeight: px(150)  // stroke-level charts

    // MARK: - Typography (DESIGN.md hierarchy on the system font, sized as
    // ratios of the 13 px reference; tabular figures through the OpenType
    // `tnum` feature, Qt 6.11 has no Font.TabularNumbers)
    // The One Hero Rule: heroMetric appears at most once per card.

    readonly property bool isMac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
    /// Chinese and Japanese glyphs need more pixels than Latin ones to stay
    /// legible.
    readonly property bool cjk: Settings.languageCode.indexOf("zh") === 0
                                || Settings.languageCode.indexOf("ja") === 0
    /// The smallest text the app draws, in logical pixels: 12 px on Windows
    /// and Linux (Windows' minimum for body text; KDE's is the same at its
    /// default font), 11 px on macOS (11 pt, Apple's smallest legible
    /// size), and 12 px everywhere for Chinese and Japanese. The scale's
    /// small steps (captions, chart labels) fall below it on a 12 px system
    /// font, so they stop here and stay subordinate by weight and colour.
    readonly property int textFloor: cjk || !isMac ? 12 : 11

    function fontPx(reference) {
        return Math.max(textFloor, Math.round(reference * scale))
    }
    /// Whether the floor lifted a reference size: such text is as large as
    /// the body, so a caption keeps its place by weight and colour instead.
    function floored(reference) {
        return Math.round(reference * scale) < textFloor
    }

    /// Page title — main view headings (26 px semibold at the reference).
    readonly property font pageTitle: ({ pixelSize: fontPx(26), weight: Font.DemiBold })
    /// Large hero metric — primary values in summary cards (28 px bold).
    readonly property font heroMetric: ({ pixelSize: fontPx(28), weight: Font.Bold,
                                          features: { "tnum": 1 } })
    /// Section headline — panel titles (15 px semibold).
    readonly property font sectionHeadline: ({ pixelSize: fontPx(15), weight: Font.DemiBold })
    /// Card metric — the value on a personal-best card (16 px semibold).
    readonly property font cardMetric: ({ pixelSize: fontPx(16), weight: Font.DemiBold,
                                          features: { "tnum": 1 } })
    /// Strip metric — inline values in detail/replay strips (20 px semibold).
    readonly property font stripMetric: ({ pixelSize: fontPx(20), weight: Font.DemiBold,
                                           features: { "tnum": 1 } })
    /// Metric label — labels beneath values (11 px medium; regular where
    /// the floor lifts it to the body's size, so it stays below the value).
    readonly property font metricLabel: ({ pixelSize: fontPx(11),
                                           weight: floored(11) ? Font.Normal : Font.Medium })
    /// Compact label — dense UI labels (10 px medium; regular where lifted).
    readonly property font compactLabel: ({ pixelSize: fontPx(10),
                                            weight: floored(10) ? Font.Normal : Font.Medium })
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

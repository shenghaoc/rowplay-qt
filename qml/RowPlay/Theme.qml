// SPDX-License-Identifier: GPL-3.0-or-later
// Central design tokens — a port of rowplay-studio's DesignTokens.swift and
// DESIGN.md ("The Erg Display" system) to Qt Quick.
//
// All views reference these tokens instead of hardcoding colors, spacing or
// typography, exactly like Studio. Divergences (docs/source-map.md):
// - the system font replaces SF Pro / SF Pro Rounded (cross-platform); the
//   DESIGN.md size and weight scale is kept, and Font.TabularNumbers replaces
//   .monospacedDigit();
// - macOS materials (.regularMaterial) become tonal opacity surfaces: Qt has
//   no system material on Linux/Windows, and DESIGN.md's flat-by-default rule
//   means opacity layers carry the depth;
// - surfaces, text, selection, separators and control fills follow the Apple
//   HIG macOS system colours (windowBackground, label / secondaryLabel,
//   selectedContentBackground, separator, controlBackground …) on every
//   platform, so the Fusion-based controls read as one Mac-class family
//   (ADR 0013). The PM5 palette and the Metric Mapping Rule are unchanged.
pragma Singleton
import QtQuick

QtObject {
    id: theme

    // MARK: - Colour scheme

    // Follows the system colour scheme; Qt.ColorScheme.Unknown (no portal /
    // platform support) resolves to light, DESIGN.md's default aesthetic.
    // ROWPLAY_FORCE_COLOR_SCHEME (Settings.colorSchemeOverride) pins the
    // scheme for tests and CI screenshots.
    readonly property bool dark: {
        const forced = Settings.colorSchemeOverride
        if (forced === "dark") {
            return true
        }
        if (forced === "light") {
            return false
        }
        return Qt.styleHints.colorScheme === Qt.Dark
    }

    // MARK: - Colour palette (DESIGN.md "The PM5 Palette")

    /// Primary brand blue — primary actions, distance, key emphasis.
    readonly property color primaryBlue: dark ? "#0A84FF" : "#0066CC"
    /// Warm comparison orange — watts, splits, secondary emphasis.
    readonly property color comparisonOrange: dark ? "#FF9F0A" : "#9A5700"
    /// Energetic green — positive deltas, success states, cadence highlights.
    readonly property color energeticGreen: dark ? "#30D158" : "#137333"
    /// Alert red — negative deltas, heart rate, finish markers.
    readonly property color alertRed: dark ? "#FF453A" : "#B3261E"
    /// Soft purple — elevation, descent, cadence accents.
    readonly property color softPurple: dark ? "#BF5AF2" : "#7B2CBF"
    /// Warm yellow — caution states, active indicators.
    readonly property color warmYellow: dark ? "#FFD60A" : "#7A5A00"

    // MARK: - Semantic metric colours (the Metric Mapping Rule: one colour
    // per metric domain, never cross-assigned)

    readonly property color metricDistance: primaryBlue
    readonly property color metricDuration: dark ? "#64D2FF" : "#007A99"
    /// Slightly lighter blue than distance in dark mode, so pace and distance
    /// stay distinguishable (DesignTokens MetricColor.pace).
    readonly property color metricPace: dark ? "#409CFF" : "#0066CC"
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
    /// flips the sign convention (watts/distance vs pace).
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

    // MARK: - Surfaces (tonal layering, no shadows — Flat-by-Default Rule)
    //
    // Apple HIG macOS system colours (ADR 0013). Secondary text keeps WCAG
    // AA (≥ 4.5:1) on every surface it is used on: #6e6e73 is 5.1:1 on the
    // window and 4.6:1 on the sidebar, #a1a1a6 is 6.5:1 / 5.9:1 in dark.
    // Tertiary text is for placeholders and decoration only (3.6:1 light).

    readonly property color windowBackground: dark ? "#1e1e1e" : "#ffffff"
    readonly property color sidebarBackground: dark ? "#262628" : "#f4f4f6"
    /// Unified toolbar above the content column (macOS 11+ window chrome).
    readonly property color toolbarBackground: dark ? "#242426" : "#fbfbfc"
    readonly property color textPrimary: dark ? "#f5f5f7" : "#1d1d1f"
    readonly property color textSecondary: dark ? "#a1a1a6" : "#6e6e73"
    readonly property color textTertiary: dark ? "#8a8a8f" : "#86868b"
    readonly property color accentColor: primaryBlue

    /// Selected sidebar row while the list has keyboard focus (white text on
    /// top: 5.0:1 light, 5.5:1 dark).
    readonly property color selectionFill: dark ? "#0a64d6" : "#0a6cdf"
    /// Selected row while focus is elsewhere (text keeps its normal colours).
    readonly property color selectionFillInactive: dark ? Qt.rgba(1, 1, 1, 0.10)
                                                         : Qt.rgba(0, 0, 0, 0.08)
    /// Hairline separators: split-view divider, toolbar edge, table rules.
    readonly property color separator: dark ? Qt.rgba(1, 1, 1, 0.10)
                                            : Qt.rgba(0, 0, 0, 0.10)
    /// Borderless-control hover and press washes (toolbar buttons, rows).
    readonly property color hoverFill: dark ? Qt.rgba(1, 1, 1, 0.06)
                                            : Qt.rgba(0, 0, 0, 0.045)
    readonly property color pressedFill: dark ? Qt.rgba(1, 1, 1, 0.12)
                                              : Qt.rgba(0, 0, 0, 0.09)
    /// Bezel controls: push buttons, text fields, pop-up buttons.
    readonly property color controlBackground: dark ? "#3a3a3c" : "#ffffff"
    readonly property color controlBorder: dark ? Qt.rgba(1, 1, 1, 0.12)
                                                : Qt.rgba(0, 0, 0, 0.16)
    /// Segmented control: recessed track and raised selection thumb.
    readonly property color segmentTrack: dark ? Qt.rgba(1, 1, 1, 0.08)
                                               : Qt.rgba(0, 0, 0, 0.06)
    readonly property color segmentThumb: dark ? "#636366" : "#ffffff"
    /// Keyboard focus ring (drawn 3 px outside the focused control).
    readonly property color focusRing: Qt.rgba(primaryBlue.r, primaryBlue.g,
                                               primaryBlue.b, 0.5)
    /// Grouped-form section box (settings).
    readonly property color groupBackground: dark ? Qt.rgba(1, 1, 1, 0.045)
                                                  : Qt.rgba(0, 0, 0, 0.028)
    /// Chart gridlines and the X axis line (quiet: data first).
    readonly property color chartGrid: dark ? Qt.rgba(1, 1, 1, 0.08)
                                            : Qt.rgba(0, 0, 0, 0.07)
    readonly property color chartAxis: dark ? Qt.rgba(1, 1, 1, 0.22)
                                            : Qt.rgba(0, 0, 0, 0.20)

    /// Subtle grouped background for panels — lighter than the window.
    readonly property color panelBackground: dark ? Qt.rgba(1, 1, 1, 0.03)
                                                  : Qt.rgba(0, 0, 0, 0.03)
    /// Card background with subtle warmth.
    readonly property color cardBackground: dark ? Qt.rgba(1, 1, 1, 0.04)
                                                  : Qt.rgba(0, 0, 0, 0.04)
    /// Active/selected card background.
    readonly property color activeCardBackground: Qt.rgba(primaryBlue.r,
                                                          primaryBlue.g,
                                                          primaryBlue.b, 0.08)
    /// Overlay backdrop (Studio: control background at 0.85) — the replay
    /// HUD's translucent material and tooltips.
    readonly property color overlayBackground: dark ? Qt.rgba(0.14, 0.14, 0.15, 0.84)
                                                    : Qt.rgba(0.98, 0.98, 0.99, 0.86)

    // MARK: - Replay materials (Phase 5a)
    //
    // The V3/V4 packs ship one neutral placeholder material; product colour
    // lives here so the 3D scene shares the shell's light/dark palettes.
    // `rowplay_viewmodel::replay::materials` maps every replayMaterialRole to
    // one of these tokens (or to the venue palette for lane paint) — QML must
    // not hardcode material hex values.

    /// Athlete skin (V4 vertex-colour skin regions).
    readonly property color replaySkin: dark ? "#c98d68" : "#d99a72"
    /// Race kit fabric (jersey / top).
    readonly property color replayFabric: dark ? "#2f6fb2" : "#2a62a0"
    /// Hair cap.
    readonly property color replayHair: dark ? "#3a2b22" : "#4a372c"
    /// Footwear soles and uppers.
    readonly property color replayFootwear: dark ? "#d8d8dc" : "#eceef0"
    /// Shorts / lower kit.
    readonly property color replayShorts: dark ? "#23262b" : "#2c3036"
    /// Kit accents and trims on the athlete.
    readonly property color replayTrim: dark ? "#ffd60a" : "#b58900"
    /// Eye whites / iris base.
    readonly property color replayEye: dark ? "#e8ecef" : "#f4f7f9"
    /// Brow / lip / nostril detail.
    readonly property color replayFaceDetail: dark ? "#8a5a44" : "#96604a"
    /// Dark equipment shells (hull underside, frames).
    readonly property color replayEquipmentDark: dark ? "#1d2126" : "#23272c"
    /// Light equipment shells (decks, fairings).
    readonly property color replayEquipmentLight: dark ? "#c9ced4" : "#e4e8ec"
    /// Metals: riggers, chains, axles, pole shafts.
    readonly property color replayEquipmentMetal: dark ? "#9aa3ab" : "#b9c1c8"
    /// Rubber: tyres, straps, heel cups.
    readonly property color replayEquipmentRubber: dark ? "#141618" : "#1a1d20"
    /// Grips: scull handles, pole grips, hoods.
    readonly property color replayEquipmentGrip: dark ? "#2b2e33" : "#33373c"
    /// Equipment trim lines and decals.
    readonly property color replayEquipmentTrim: dark ? "#ff9f0a" : "#9a5700"

    // MARK: - Spacing scale (8-point soft grid)

    readonly property int spacingXxSmall: 2    // hairline gaps
    readonly property int spacingXSmall: 4     // tight gaps within components
    readonly property int spacingSmall: 6      // compact gaps
    readonly property int spacingMedium: 8     // default inner padding
    readonly property int spacingLarge: 12     // standard component spacing
    readonly property int spacingXLarge: 16    // section-level spacing
    readonly property int spacingXxLarge: 20   // generous section gaps
    readonly property int spacingXxxLarge: 24  // major section separation

    // MARK: - Corner radii

    readonly property int radiusSmall: 6   // small badges, tags
    readonly property int radiusMedium: 8  // cards, panels
    readonly property int radiusLarge: 12  // large cards, overlays
    readonly property int radiusXLarge: 16 // hero cards

    // MARK: - Chart sizing

    readonly property int chartHeight: 220        // dashboard-level charts
    readonly property int chartStrokeHeight: 150  // stroke-level charts

    // MARK: - Control metrics (HIG macOS regular control size)

    readonly property int controlHeight: 28     // push / pop-up / field / segment
    readonly property int toolbarHeight: 52     // unified toolbar
    readonly property int sidebarRowHeight: 48  // two-line sidebar row

    // MARK: - Typography (DESIGN.md hierarchy; system font, weights kept).
    // The One Hero Rule: heroMetric appears at most once per card.

    /// Page title — main view headings (largeTitle semibold ≈ 26px).
    readonly property font pageTitle: ({ pixelSize: 26, weight: Font.DemiBold })
    /// Large hero metric — primary values in summary cards (title bold, 28px).
    readonly property font heroMetric: ({ pixelSize: 28, weight: Font.Bold })
    /// Section headline — panel titles (headline semibold, 15px).
    readonly property font sectionHeadline: ({ pixelSize: 15, weight: Font.DemiBold })
    /// Metric value — data values in badges and cards (callout semibold, 13px).
    readonly property font metricValue: ({ pixelSize: 13, weight: Font.DemiBold,
                                           features: Font.TabularNumbers })
    /// Strip metric — inline values in detail/replay strips (20px semibold,
    /// tabular figures on the system font; a monospace family made values
    /// run into the next label).
    readonly property font stripMetric: ({ pixelSize: 20, weight: Font.DemiBold,
                                           features: Font.TabularNumbers })
    /// Emphasised body — sidebar row titles (13px medium).
    readonly property font bodyEmphasized: ({ pixelSize: 13, weight: Font.Medium })
    /// Tabular body — table cells and trailing row values (13px semibold).
    readonly property font tabularBody: ({ pixelSize: 13, weight: Font.DemiBold,
                                           features: Font.TabularNumbers })
    /// Sidebar section header — sentence case, bold, small (macOS 11+).
    readonly property font sidebarSection: ({ pixelSize: 11, weight: Font.Bold })
    /// Grouped-form section title (HIG "headline", 13px bold).
    readonly property font groupTitle: ({ pixelSize: 13, weight: Font.Bold })
    /// Metric label — labels beneath values (caption2 medium, 11px).
    readonly property font metricLabel: ({ pixelSize: 11, weight: Font.Medium })
    /// Compact label — dense UI labels (10px medium).
    readonly property font compactLabel: ({ pixelSize: 10, weight: Font.Medium })
    /// Body text (13px regular).
    readonly property font body: ({ pixelSize: 13, weight: Font.Normal })
    /// Subheadline metadata (Studio .subheadline ≈ 12px).
    readonly property font subheadline: ({ pixelSize: 12, weight: Font.Normal })
}

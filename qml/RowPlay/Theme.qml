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
//   means opacity layers carry the depth.
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

    readonly property color windowBackground: dark ? "#232629" : "#ffffff"
    readonly property color sidebarBackground: dark ? "#2a2e32" : "#f7f7f7"
    readonly property color textPrimary: dark ? "#f2f4f6" : "#1a1c1e"
    readonly property color textSecondary: dark ? "#a8adb3" : "#5c6166"
    readonly property color textTertiary: dark ? "#7c8187" : "#8a8f95"
    readonly property color accentColor: primaryBlue

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
    /// Overlay backdrop (Studio: control background at 0.85).
    readonly property color overlayBackground: dark ? Qt.rgba(0.16, 0.17, 0.18, 0.85)
                                                    : Qt.rgba(1, 1, 1, 0.85)

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
    /// Strip metric — inline values in detail/replay strips (18px semibold mono).
    readonly property font stripMetric: ({ pixelSize: 18, weight: Font.DemiBold,
                                           family: "monospace",
                                           features: Font.TabularNumbers })
    /// Metric label — labels beneath values (caption2 medium, 11px).
    readonly property font metricLabel: ({ pixelSize: 11, weight: Font.Medium })
    /// Compact label — dense UI labels (10px medium).
    readonly property font compactLabel: ({ pixelSize: 10, weight: Font.Medium })
    /// Body text (13px regular).
    readonly property font body: ({ pixelSize: 13, weight: Font.Normal })
    /// Subheadline metadata (Studio .subheadline ≈ 12px).
    readonly property font subheadline: ({ pixelSize: 12, weight: Font.Normal })
}

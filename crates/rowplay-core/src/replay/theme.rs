// SPDX-License-Identifier: GPL-3.0-or-later
//! 2D replay palettes (web `replay/renderer.ts` `COLORS_*` / `VENUES_*`).
//!
//! Canvas can't read CSS custom properties, so the web mirrors `app.css`
//! here; the Qt Quick Canvas in Phase 4 needs the same constants, and the
//! `live`/`ghost` values must stay in sync with the web stylesheet. Values
//! are asserted against the `palettes` block of
//! `tests/fixtures/replay-current-main-2d.json`.

use crate::models::Sport;

/// Overlay / annotation colours for the 2D replay (web `CanvasColors`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasColors {
    /// Major timeline tick.
    pub tick_major: &'static str,
    /// Minor timeline tick.
    pub tick_minor: &'static str,
    /// Timeline tick label.
    pub tick_text: &'static str,
    /// Lane divider.
    pub lane_line: &'static str,
    /// Bib / avatar chip fill.
    pub bib_fill: &'static str,
    /// Bib label.
    pub bib_text: &'static str,
    /// Bib position dot.
    pub bib_dot: &'static str,
    /// Finish checker dark square.
    pub finish_dark: &'static str,
    /// Finish checker light square.
    pub finish_light: &'static str,
    /// Pace-label background.
    pub label_bg: &'static str,
    /// Pace-label text.
    pub label_text: &'static str,
    /// Course surface fill.
    pub course_fill: &'static str,
    /// Athlete skin, kept distinct from both live and ghost race-kit colours.
    pub live: &'static str,
    /// Ghost lane kit.
    pub ghost: &'static str,
    /// Sky gradient top.
    pub sky_top: &'static str,
    /// Sky gradient bottom.
    pub sky_bottom: &'static str,
    /// Distance-marker cap.
    pub marker_cap: &'static str,
    /// Wake foam.
    pub foam: &'static str,
    /// Contact shadow.
    pub shadow: &'static str,
    /// Athlete skin.
    pub skin: &'static str,
    /// Recessed / far-side skin used to preserve depth in the silhouette.
    pub skin_shade: &'static str,
    /// Hair and helmet-detail colour.
    pub hair: &'static str,
    /// Shoes and contact-point footwear.
    pub shoe: &'static str,
}

/// One sport's venue palette (web `VenuePalette`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenuePalette {
    /// Sky gradient top.
    pub sky_top: &'static str,
    /// Sky gradient horizon.
    pub sky_horizon: &'static str,
    /// Atmospheric haze band.
    pub haze: &'static str,
    /// Sun disc / glow.
    pub sun: &'static str,
    /// Distant ridge silhouette.
    pub ridge_far: &'static str,
    /// Near ridge silhouette.
    pub ridge_near: &'static str,
    /// Distant foliage band.
    pub foliage_far: &'static str,
    /// Near foliage band.
    pub foliage_near: &'static str,
    /// Boathouse / structure body.
    pub structure: &'static str,
    /// Structure shaded face.
    pub structure_shade: &'static str,
    /// Structure lit face.
    pub structure_light: &'static str,
    /// Ground gradient top.
    pub ground_top: &'static str,
    /// Ground gradient middle.
    pub ground_mid: &'static str,
    /// Ground gradient bottom.
    pub ground_bottom: &'static str,
    /// Lane surface line.
    pub surface_line: &'static str,
    /// Surface specular highlight.
    pub surface_highlight: &'static str,
    /// Surface shadow edge.
    pub surface_shadow: &'static str,
    /// Distance marker accent.
    pub marker: &'static str,
    /// Safety / buoy base colour.
    pub safety: &'static str,
    /// Safety highlight.
    pub safety_light: &'static str,
}

/// Light-theme overlay colours (web `COLORS_LIGHT`).
pub const COLORS_LIGHT: CanvasColors = CanvasColors {
    tick_major: "#bed0d7",
    tick_minor: "#d0dbdf",
    tick_text: "#4a6470",
    lane_line: "#bed0d7",
    bib_fill: "#f0f4f6",
    bib_text: "#0f2a36",
    bib_dot: "#f7fafb",
    finish_dark: "#0f2a36",
    finish_light: "#f7fafb",
    label_bg: "#f7fafb",
    label_text: "#0f2a36",
    course_fill: "#e4ecef",
    live: "#5240ce",
    ghost: "#176b8c",
    sky_top: "#f2f7f9",
    sky_bottom: "#e3edf1",
    marker_cap: "#9fb8c2",
    foam: "#ffffff",
    shadow: "#0f2a36",
    skin: "#bb7053",
    skin_shade: "#8e4f3d",
    hair: "#263840",
    shoe: "#172a33",
};

/// Dark-theme overlay colours (web `COLORS_DARK`).
pub const COLORS_DARK: CanvasColors = CanvasColors {
    tick_major: "#3d505a",
    tick_minor: "#2e3d45",
    tick_text: "#8aa2ac",
    lane_line: "#3d505a",
    bib_fill: "#1c2a32",
    bib_text: "#dce6ea",
    bib_dot: "#0f2a36",
    finish_dark: "#dce6ea",
    finish_light: "#0f2a36",
    label_bg: "#0f2a36",
    label_text: "#dce6ea",
    course_fill: "#142128",
    live: "#8c7cf0",
    ghost: "#3aa8cc",
    sky_top: "#0e1d26",
    sky_bottom: "#0a151c",
    marker_cap: "#3d505a",
    foam: "#bcd3dd",
    shadow: "#000000",
    skin: "#e2a27f",
    skin_shade: "#ad6c54",
    hair: "#78919c",
    shoe: "#d9e4e8",
};

const VENUES_LIGHT_ROWER: VenuePalette = VenuePalette {
    sky_top: "#4d86a8",
    sky_horizon: "#f2d29a",
    haze: "#f3e2bc",
    sun: "#ffe7b0",
    ridge_far: "#9aae90",
    ridge_near: "#4a6d56",
    foliage_far: "#5a7a62",
    foliage_near: "#2a5244",
    structure: "#ece6d9",
    structure_shade: "#8c7b67",
    structure_light: "#ffd68a",
    ground_top: "#4a92a3",
    ground_mid: "#1d6172",
    ground_bottom: "#0c3a48",
    surface_line: "#9fd6df",
    surface_highlight: "#e8f6f7",
    surface_shadow: "#082a37",
    marker: "#ef5b42",
    safety: "#d9e7e7",
    safety_light: "#ffffff",
};

const VENUES_LIGHT_SKIERG: VenuePalette = VenuePalette {
    sky_top: "#357db3",
    sky_horizon: "#dcecf5",
    haze: "#f6fbfd",
    sun: "#fff5cf",
    ridge_far: "#b8cedb",
    ridge_near: "#66899e",
    foliage_far: "#43675d",
    foliage_near: "#244a42",
    structure: "#e7edf1",
    structure_shade: "#607887",
    structure_light: "#fff1b2",
    ground_top: "#f2f7fa",
    ground_mid: "#d5e4ee",
    ground_bottom: "#b0c9d8",
    surface_line: "#8eb5c8",
    surface_highlight: "#ffffff",
    surface_shadow: "#6f96ab",
    marker: "#6d5ef5",
    safety: "#1e6292",
    safety_light: "#f5fbfd",
};

const VENUES_LIGHT_BIKE: VenuePalette = VenuePalette {
    // Bright indoor timber velodrome; the "sky" family is the roof daylight.
    sky_top: "#edf3f4",
    sky_horizon: "#cbd8da",
    haze: "#f8f4e8",
    sun: "#fff7d8",
    ridge_far: "#b6c2c4",
    ridge_near: "#87969b",
    foliage_far: "#6f817a",
    foliage_near: "#4e6d63",
    structure: "#d9e1e2",
    structure_shade: "#596970",
    structure_light: "#fff6d4",
    ground_top: "#e0c39a",
    ground_mid: "#c99b68",
    ground_bottom: "#91623e",
    surface_line: "#f3eee4",
    surface_highlight: "#fff8e8",
    surface_shadow: "#503c2f",
    marker: "#c83f38",
    safety: "#2f7298",
    safety_light: "#f8f5ed",
};

const VENUES_DARK_ROWER: VenuePalette = VenuePalette {
    sky_top: "#071724",
    sky_horizon: "#294f62",
    haze: "#7a9499",
    sun: "#f0c67b",
    ridge_far: "#3a5654",
    ridge_near: "#1d3f39",
    foliage_far: "#2a4d44",
    foliage_near: "#12332d",
    structure: "#8c908c",
    structure_shade: "#3b4648",
    structure_light: "#f0b65c",
    ground_top: "#1f5a6c",
    ground_mid: "#0f3644",
    ground_bottom: "#061c26",
    surface_line: "#5aa3b4",
    surface_highlight: "#b6dce2",
    surface_shadow: "#03141c",
    marker: "#ef6a4e",
    safety: "#60777c",
    safety_light: "#c8d9db",
};

const VENUES_DARK_SKIERG: VenuePalette = VenuePalette {
    sky_top: "#061522",
    sky_horizon: "#28516a",
    haze: "#7795a5",
    sun: "#e8d5a1",
    ridge_far: "#60798a",
    ridge_near: "#334f60",
    foliage_far: "#28473f",
    foliage_near: "#142f2b",
    structure: "#71838c",
    structure_shade: "#293c47",
    structure_light: "#ffe099",
    ground_top: "#cfe3ec",
    ground_mid: "#9fbfd0",
    ground_bottom: "#6e93a6",
    surface_line: "#6f9eb3",
    surface_highlight: "#f1f7f9",
    surface_shadow: "#456c80",
    marker: "#8b7cf5",
    safety: "#1f5f85",
    safety_light: "#d7e8ee",
};

const VENUES_DARK_BIKE: VenuePalette = VenuePalette {
    sky_top: "#1b2934",
    sky_horizon: "#40515b",
    haze: "#66767b",
    sun: "#f2c981",
    ridge_far: "#45535a",
    ridge_near: "#2c3b43",
    foliage_far: "#3d514a",
    foliage_near: "#26473e",
    structure: "#849198",
    structure_shade: "#25333c",
    structure_light: "#f4d38c",
    ground_top: "#9f7650",
    ground_mid: "#775337",
    ground_bottom: "#3f2d23",
    surface_line: "#d9d4ca",
    surface_highlight: "#ebddc5",
    surface_shadow: "#171412",
    marker: "#ef5f53",
    safety: "#5fa4c4",
    safety_light: "#e8e3d8",
};

/// Light-theme venue palette per sport (web `VENUES_LIGHT`).
#[must_use]
pub fn venues_light(sport: Sport) -> &'static VenuePalette {
    match sport {
        Sport::Rower => &VENUES_LIGHT_ROWER,
        Sport::Skierg => &VENUES_LIGHT_SKIERG,
        Sport::Bike => &VENUES_LIGHT_BIKE,
    }
}

/// Dark-theme venue palette per sport (web `VENUES_DARK`).
#[must_use]
pub fn venues_dark(sport: Sport) -> &'static VenuePalette {
    match sport {
        Sport::Rower => &VENUES_DARK_ROWER,
        Sport::Skierg => &VENUES_DARK_SKIERG,
        Sport::Bike => &VENUES_DARK_BIKE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_cover_every_sport_and_theme() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            for palette in [venues_light(sport), venues_dark(sport)] {
                for value in [palette.sky_top, palette.marker, palette.safety_light] {
                    assert!(value.starts_with('#') && value.len() == 7, "{value}");
                }
            }
        }
        assert_ne!(venues_light(sport_default()), venues_dark(sport_default()));
    }

    fn sport_default() -> Sport {
        Sport::Rower
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! The replay material roles and their palette resolution (Phase 5a spec R3).
//!
//! The V3 pack ships one neutral placeholder material on purpose: product
//! colour lives outside the GLB. The eleven `replayMaterialRole` values of
//! `renderer3dAssets.ts` plus the three extra V4 surface roles form one enum
//! here; each role resolves to a `Theme.qml` token key (the single colour
//! source for shell chrome) or, for lane paint, to the venue palette data in
//! [`super::palette`]. QML binds `Theme[spec.theme_key]` and only the PBR
//! scalars and the ghost opacity come from Rust — no hex literal for a
//! material colour appears in `qml/`.

/// Every material role the replay renderer resolves, V3 pack order first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialRole {
    /// V3 `athlete-skin`.
    AthleteSkin,
    /// V3 `athlete-fabric`.
    AthleteFabric,
    /// V3 `athlete-hair`.
    AthleteHair,
    /// V3 `athlete-footwear`.
    AthleteFootwear,
    /// V3 `equipment-painted` — lane paint, resolved per sport at runtime.
    EquipmentPainted,
    /// V3 `equipment-dark`.
    EquipmentDark,
    /// V3 `equipment-light`.
    EquipmentLight,
    /// V3 `equipment-metal`.
    EquipmentMetal,
    /// V3 `equipment-rubber`.
    EquipmentRubber,
    /// V3 `equipment-grip`.
    EquipmentGrip,
    /// V3 `equipment-trim`.
    EquipmentTrim,
    /// V4-only surface role `athlete-shorts`.
    AthleteShorts,
    /// V4-only surface role `athlete-trim`.
    AthleteTrim,
    /// V4-only surface role `athlete-eye`.
    AthleteEye,
    /// V4-only surface role `athlete-face-detail`.
    AthleteFaceDetail,
}

/// All roles, V3 pack order then the V4 additions.
pub const ALL_ROLES: [MaterialRole; 15] = [
    MaterialRole::AthleteSkin,
    MaterialRole::AthleteFabric,
    MaterialRole::AthleteHair,
    MaterialRole::AthleteFootwear,
    MaterialRole::EquipmentPainted,
    MaterialRole::EquipmentDark,
    MaterialRole::EquipmentLight,
    MaterialRole::EquipmentMetal,
    MaterialRole::EquipmentRubber,
    MaterialRole::EquipmentGrip,
    MaterialRole::EquipmentTrim,
    MaterialRole::AthleteShorts,
    MaterialRole::AthleteTrim,
    MaterialRole::AthleteEye,
    MaterialRole::AthleteFaceDetail,
];

/// The eleven roles the V3 pack may declare on its meshes.
pub const V3_ROLES: [MaterialRole; 11] = [
    MaterialRole::AthleteSkin,
    MaterialRole::AthleteFabric,
    MaterialRole::AthleteHair,
    MaterialRole::AthleteFootwear,
    MaterialRole::EquipmentPainted,
    MaterialRole::EquipmentDark,
    MaterialRole::EquipmentLight,
    MaterialRole::EquipmentMetal,
    MaterialRole::EquipmentRubber,
    MaterialRole::EquipmentGrip,
    MaterialRole::EquipmentTrim,
];

/// Ghost equipment transparency (Phase 5a spec R3.2). The V4 athlete stays
/// opaque even as a ghost — the contract's depth rule — so only equipment
/// and effects use this.
pub const GHOST_EQUIPMENT_OPACITY: f32 = 0.45;

impl MaterialRole {
    /// Parses a `replayMaterialRole` id from a glTF extras string.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        ALL_ROLES.into_iter().find(|role| role.as_id() == id)
    }

    /// The wire id exactly as the packs spell it.
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::AthleteSkin => "athlete-skin",
            Self::AthleteFabric => "athlete-fabric",
            Self::AthleteHair => "athlete-hair",
            Self::AthleteFootwear => "athlete-footwear",
            Self::EquipmentPainted => "equipment-painted",
            Self::EquipmentDark => "equipment-dark",
            Self::EquipmentLight => "equipment-light",
            Self::EquipmentMetal => "equipment-metal",
            Self::EquipmentRubber => "equipment-rubber",
            Self::EquipmentGrip => "equipment-grip",
            Self::EquipmentTrim => "equipment-trim",
            Self::AthleteShorts => "athlete-shorts",
            Self::AthleteTrim => "athlete-trim",
            Self::AthleteEye => "athlete-eye",
            Self::AthleteFaceDetail => "athlete-face-detail",
        }
    }

    /// Whether the V3 rig pack may declare this role on a mesh.
    #[must_use]
    pub const fn is_v3(self) -> bool {
        matches!(
            self,
            Self::AthleteSkin
                | Self::AthleteFabric
                | Self::AthleteHair
                | Self::AthleteFootwear
                | Self::EquipmentPainted
                | Self::EquipmentDark
                | Self::EquipmentLight
                | Self::EquipmentMetal
                | Self::EquipmentRubber
                | Self::EquipmentGrip
                | Self::EquipmentTrim
        )
    }

    /// The resolved material parameters for this role.
    #[must_use]
    pub const fn spec(self) -> RoleSpec {
        match self {
            Self::AthleteSkin => RoleSpec {
                theme_key: Some("replaySkin"),
                venue: false,
                metalness: 0.0,
                roughness: 0.55,
            },
            Self::AthleteFabric => RoleSpec {
                theme_key: Some("replayFabric"),
                venue: false,
                metalness: 0.0,
                roughness: 0.8,
            },
            Self::AthleteHair => RoleSpec {
                theme_key: Some("replayHair"),
                venue: false,
                metalness: 0.0,
                roughness: 0.45,
            },
            Self::AthleteFootwear => RoleSpec {
                theme_key: Some("replayFootwear"),
                venue: false,
                metalness: 0.0,
                roughness: 0.6,
            },
            Self::AthleteShorts => RoleSpec {
                theme_key: Some("replayShorts"),
                venue: false,
                metalness: 0.0,
                roughness: 0.8,
            },
            Self::AthleteTrim => RoleSpec {
                theme_key: Some("replayTrim"),
                venue: false,
                metalness: 0.1,
                roughness: 0.4,
            },
            Self::AthleteEye => RoleSpec {
                theme_key: Some("replayEye"),
                venue: false,
                metalness: 0.0,
                roughness: 0.15,
            },
            Self::AthleteFaceDetail => RoleSpec {
                theme_key: Some("replayFaceDetail"),
                venue: false,
                metalness: 0.0,
                roughness: 0.6,
            },
            Self::EquipmentPainted => RoleSpec {
                theme_key: None,
                venue: true,
                metalness: 0.05,
                roughness: 0.35,
            },
            Self::EquipmentDark => RoleSpec {
                theme_key: Some("replayEquipmentDark"),
                venue: false,
                metalness: 0.2,
                roughness: 0.5,
            },
            Self::EquipmentLight => RoleSpec {
                theme_key: Some("replayEquipmentLight"),
                venue: false,
                metalness: 0.1,
                roughness: 0.4,
            },
            Self::EquipmentMetal => RoleSpec {
                theme_key: Some("replayEquipmentMetal"),
                venue: false,
                metalness: 0.9,
                roughness: 0.25,
            },
            Self::EquipmentRubber => RoleSpec {
                theme_key: Some("replayEquipmentRubber"),
                venue: false,
                metalness: 0.0,
                roughness: 0.9,
            },
            Self::EquipmentGrip => RoleSpec {
                theme_key: Some("replayEquipmentGrip"),
                venue: false,
                metalness: 0.0,
                roughness: 0.85,
            },
            Self::EquipmentTrim => RoleSpec {
                theme_key: Some("replayEquipmentTrim"),
                venue: false,
                metalness: 0.3,
                roughness: 0.4,
            },
        }
    }
}

/// Resolved material parameters for one role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleSpec {
    /// The `Theme.qml` colour property, or `None` when the colour is venue
    /// data (lane paint) resolved through [`super::palette`].
    pub theme_key: Option<&'static str>,
    /// Lane paint: take the sport's venue `marker`/live colour instead.
    pub venue: bool,
    /// Principled metalness 0..1.
    pub metalness: f32,
    /// Principled roughness 0..1.
    pub roughness: f32,
}

impl RoleSpec {
    /// Opacity for this role when rendering a ghost.
    #[must_use]
    pub fn ghost_opacity(self, role: MaterialRole) -> f32 {
        // The V4 depth contract keeps deforming skins opaque; equipment and
        // effects go translucent.
        if role.is_v3()
            && !matches!(
                role,
                MaterialRole::AthleteSkin
                    | MaterialRole::AthleteFabric
                    | MaterialRole::AthleteHair
                    | MaterialRole::AthleteFootwear
            )
        {
            GHOST_EQUIPMENT_OPACITY
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::glb;

    #[test]
    fn ids_round_trip_and_match_the_v3_pack_order() {
        for (role, id) in V3_ROLES.iter().zip(glb::V3_MATERIAL_ROLES) {
            assert_eq!(role.as_id(), id);
            assert_eq!(MaterialRole::from_id(id), Some(*role));
        }
        assert_eq!(ALL_ROLES.len(), 15);
        assert!(MaterialRole::from_id("athlete-shorts").is_some());
        assert!(MaterialRole::from_id("nope").is_none());
    }

    #[test]
    fn every_role_resolves_to_a_theme_key_or_the_venue_palette() {
        for role in ALL_ROLES {
            let spec = role.spec();
            assert!(
                spec.theme_key.is_some() ^ spec.venue,
                "{role:?} must resolve to exactly one colour source"
            );
            assert!((0.0..=1.0).contains(&spec.metalness));
            assert!((0.0..=1.0).contains(&spec.roughness));
        }
    }

    #[test]
    fn theme_keys_exist_in_theme_qml() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("qml")
            .join("RowPlay")
            .join("Theme.qml");
        let theme = std::fs::read_to_string(&path).expect("Theme.qml");
        for role in ALL_ROLES {
            let Some(key) = role.spec().theme_key else {
                continue;
            };
            assert!(
                theme.contains(&format!("property color {key}:")),
                "Theme.qml is missing the replay material token {key} (role {role:?})"
            );
        }
    }

    #[test]
    fn ghosts_keep_the_athlete_opaque() {
        for role in ALL_ROLES {
            let opacity = role.spec().ghost_opacity(role);
            if matches!(
                role,
                MaterialRole::AthleteSkin
                    | MaterialRole::AthleteFabric
                    | MaterialRole::AthleteHair
                    | MaterialRole::AthleteFootwear
                    | MaterialRole::AthleteShorts
                    | MaterialRole::AthleteTrim
                    | MaterialRole::AthleteEye
                    | MaterialRole::AthleteFaceDetail
            ) {
                assert_eq!(opacity, 1.0, "{role:?} skin stays opaque as a ghost");
            } else {
                assert_eq!(opacity, GHOST_EQUIPMENT_OPACITY, "{role:?}");
            }
        }
    }
}

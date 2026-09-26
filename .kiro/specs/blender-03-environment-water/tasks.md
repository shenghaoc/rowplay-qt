# Blender Phase 3 - environment and water

Owner-approved scope: finish the water surface and give the rowing basin a
near bank, mid-distance mass and a far-bank silhouette, so the replay stops
reading as a boat inside an empty test arena. Direction C and its lighting
are approved and stay (Phase 1): overcast light, the independently authored
blue hour, the probes, the key light and the fog are not redone. The athlete
and the data stay visually primary.

Unchanged by design: the 225-float frame, replay state, athlete pose, camera,
oar truth, the shell and oar rig contract, the distance-to-loop mapping and the
circular course. The world may respond to the replay but must not invent a
second reading of distance or speed: the water is fixed in the world, and the
boat moves through it.

Out of scope, left for their own phases: course dressing (start pontoons,
finish tower, distance markers, officials' structures, boards, lane
infrastructure), wake and foam, the athlete, and the ReplayScene refactor.
#121, #129 and #130 stay open.

- [x] Restack: #132 did not contain #131's two newest commits; the branch sits on
  #131's head with #132's two commits cherry-picked on top (clean).
- [x] Capture the Phase 2 scene at the approved moment (demo 1001, 208.829 s,
  mid-drive) natively on Metal, light and blue hour, Low to Ultra, and name
  the three most visible environment deficiencies.
- [x] Set the budgets before building (`docs/blender-audit.md`, "Phase 3").
- [x] Concept in Blender, iterated by eye: chase-camera view, eight chase views
  round the lap, a wider context view, light and blue hour.
- [x] Classify every new asset: the water normal is procedural (`water.py`); the
  terrain, far bank, woodland masses, vegetation variants and placements are
  modelled, with `rowing-environment.blend` as their source.
- [x] Water: a seeded periodic wind-sea spectrum on an 8 m tile, 360 wave
  vectors, 6 cm to 2.8 m, RMS slope 0.11 chosen in Qt; world-fixed, no
  scrolling, no speed term.
- [x] Near bank: woodland banks (earth face and grass verge, 0.7-1.9 m), the
  campus quay (a 0.18 m retaining edge meeting the web launch dock) and the
  wetland's shallow reed shore.
- [x] Mid-distance: eight woodland stands with continuous canopy masses, a
  campus poplar row, parkland trees, shrubs and reeds: six instanced variants,
  294 placements over four tiers.
- [x] Far bank: a forest belt with an irregular crown line and distant hills.
- [x] Export and validate deterministically (`export_environment.py`): budgets,
  the waterline, the retained structures' footprints and ground heights, tiers.
- [x] Draw it in Qt from build-time data (`build.rs` generates
  `EnvironmentScene`); no runtime walker. The existing venue walk hides the web
  rower venue's land and vegetation; its structures stay.
- [x] The terrain receives the key light's shadow, as the web's banks do.
- [x] Evidence: Blender overview, bank and water views, light and blue hour;
  Qt before/after at Medium in both schemes, four tiers, the lap, native
  environment crops, water crops, a 12 s motion sequence.
- [x] Invariants: all 18 controlled states per scheme unchanged; the same replay
  state renders identical pixels at different wall-clock times.
- [x] Performance on the Apple M5: steady GUI intervals at Medium, High and
  Ultra, back to back against the parent; CPU and memory samples.
- [x] Determinism: the water regenerates byte for byte; the export from the
  unchanged `.blend` reproduces the committed GLB and placements.
- [x] Tests: the manifest's budgets and source hash, the placement invariants,
  the water tile against `RowingWater.qml`, the spectrum's lattice guards, the
  export helpers.
- [x] Records: this file, `docs/blender-audit.md`, the roadmap, the source map,
  the bridge notes, `tools/blender/README.md`, `ASSET_PROVENANCE.md`, ADR 0016.

Found and left alone: the phase-correlation parallax measure did not separate
depths in this framing and is not used; GPU time was not measured (the Metal
HUD logged nothing, and the bench is vsync-capped on Cocoa); the water tile
repeats every 8 m when looked at from above.

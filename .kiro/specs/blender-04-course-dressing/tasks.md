# Blender Phase 4 - course dressing and venue furniture

Owner-approved scope: make the basin read as a rowing venue. Where the
course is, where crews launch, where the timing and finish infrastructure
stands, what gives the scene human scale, and what marks the lap, with
clusters and identifiable zones rather than a filled shoreline. Generic,
not branded. Phase 3's environment (the authored banks, the mid-distance
vegetation, the far bank, the world-fixed water and its normal, the
declarative generation, the terrain's shadow role) is not regressed, and
the structures stand on its terrain.

Unchanged by design: the 225-float frame, the camera truth, the athlete
pose, the seat and pelvis anchor, the oar truth, the shell contract, the
course-distance mapping, the circular replay and the workout speed. Every
structure is stationary world geometry; nothing scrolls or animates.

Out of scope, left for their own phases: the athlete (#130 stays open; the
skeleton, the cockpit fit, the shell's width, the seat, the oar rig and any
athlete-reference comparison are Phase 5/6), #129, wake and foam, and the
ReplayScene refactor.

- [x] Branch `blender/04-course-dressing` in its own worktree. `docs/blender-roadmap-v2`
  had not merged and the Blender stack (#123, #131, #132, #133) was still
  open, so the branch sits on `blender/03-environment-water`'s head, not on
  `main`, which has no Phase 3 environment to dress.
- [x] Audit before modelling: capture every retained web structure natively
  (the approved moment, eight loop positions, the launch, finish, bridge,
  far-bank and wetland zones, the island with a ghost, Low to Ultra) and
  classify each (`docs/blender-audit.md`, "Phase 4"): the tower, the start
  pontoons, the launch dock, the distance posts, the three campus buildings,
  the bridge, the boardwalk and the hide are rebuilt; the island is replaced
  by an authored one; the overlays hidden since Phases 1 and 3 stay removed.
- [x] Decide the layout: the web's zones stay (campus and launch at 8-56
  degrees, the finish line at 52, the bridge at 148, the wetland at
  300-358); the start and finish are one line, marked by the tower's boom,
  the island's jetty and two buoys; distance boards at 250, 500 and 750 m
  from it; a coaching pontoon and launch on the far bank at 264 degrees.
- [x] Set the budgets before building, from the Phase 3 scene
  (`docs/blender-audit.md`, "Phase 4"): 5,000 triangles per structure,
  30,000 for all, 600 per furniture variant, 10/40/80/120 instances and
  4k/10k/20k/30k furniture triangles by tier, no textures, the pack at 8 MiB.
- [x] Model in Blender and judge from the chase camera: the concept was
  rendered from the approved view, the zone views and close-ups
  (`review_environment.py` draws the dressing since this phase), then in Qt.
- [x] Classify every new asset: `rowing-dressing.blend` is a modelled source
  (ADR 0016, on the Phase 3 precedent); `rowing-dressing.glb` and
  `dressing.json` are exported from it by `export_dressing.py`; repeated
  furniture is instanced from six variants, placed in tier collections.
- [x] The finish tower: a judges' tower on a lattice frame with an open
  stair, a mid landing, a glazed cabin with a gallery, a finish-line boom
  over the water carrying a camera housing and a plain board, a mast with
  a plain flag; the one vertical, at every tier.
- [x] The finish line: the tower's boom, the start jetty from the island's
  beach to r 21.6 m with its aligner's seat and finish post, two finish
  buoys at r 22.25 and 34.2 m; every tier.
- [x] The launch zone: a floating launch pontoon (8 m, freeboard 0.15 m,
  cleats, fenders, piles, a gangway from the quay), an upturned single on
  slings, bollards and a life ring at the quay.
- [x] The campus: a boathouse with three bay doors facing the water, a
  clubhouse with a glazed front and a terrace, a two-storey regatta office
  with a balcony, two flagpoles with plain flags.
- [x] The lap: a footbridge from the island to the bank at 148 degrees
  (A-frame piers outside the lanes, 4.9 m clearance, handrails, stairs to
  the beach); numbered boards at 142, 232 and 322 degrees with block
  numerals in geometry; a coaching pontoon with a moored coaching launch at
  264 degrees.
- [x] The quiet bank: the wetland boardwalk on piles with a handrail, ramps
  and a spur to a timber hide with a slot window.
- [x] The island: an authored lawn dome with a beach and a sunken skirt in
  the dressing pack; nine plants from the environment's own variants on its
  lawn, added to `rowing-environment.blend` in place (the environment GLB
  is byte-identical; only the placements changed). The environment's export
  admits the island's lawn and reads the dressing's footprints instead of
  the web venue's.
- [x] Export and validate deterministically (`export_dressing.py`): budgets,
  the lane band, terrain contact per structure kind, the bridge's landing,
  the island's extent, the plants, the tiers; primitives matched to their
  material slots by the centroids of their triangles.
- [x] `canonicalize_pack` sorts a shared index accessor once (Blender
  writes one for primitives of identical topology) and requires a shared
  vertex count; tested.
- [x] Draw it in Qt from build-time data: `replay::dressing::validate_dressing`
  gates the GLB against the placements, `build.rs` generates
  `DressingScene` (one Model per structure with one material per primitive
  and its shadow roles; one instanced Model per variant, hidden at a tier
  with no instance), `RowingDressing.qml` declares the six class materials.
  The web venue's structures and island are hidden by the existing walk's
  name list; nothing of the rower bake is drawn any more, and no walker was
  added or broadened.
- [x] Tier policy: every structure at every tier (Low keeps the silhouettes
  the web left it without: the tower, the jetty, the pontoons, the bridge);
  the furniture by tier, 2 / 10 / 18 / 24 instances; the structures cast
  and receive the key light's shadow at High and Ultra, the furniture casts
  none.
- [x] Evidence: Blender overview, close-ups, plan and zone views, light and
  blue hour; Qt before/after at Medium in both schemes, four tiers, the
  lap, the launch, finish, bridge, far-bank and wetland zones, the island
  with a ghost; a 12 s motion sequence.
- [x] Invariants: all 18 controlled states per scheme and the 33 lap and zone
  states match the Phase 3 parent exactly; the same replay state renders 0
  differing pixels at two wall-clock times, before and after.
- [x] Performance on the Apple M5: steady GUI intervals at Medium, High and
  Ultra in light and Medium in blue hour, back to back against the Phase 3
  parent; CPU and memory samples. One 2.7 s interval in the first Phase 4
  Medium window did not recur in two reruns per side; recorded as
  unexplained.
- [x] Determinism: the export from the unchanged `.blend` reproduces the
  committed GLB and placements, twice; the environment's re-export is
  byte-identical.
- [x] Tests: the manifest's budgets and source hash, the placements'
  invariants, the validator's refusals, the export helpers, the shared
  index accessor.
- [x] Records: this file, `docs/blender-audit.md`, the roadmap, the source
  map, the bridge notes, `tools/blender/README.md`, `ASSET_PROVENANCE.md`,
  ADR 0016 (amended in place), `AGENTS.md`, the asset inventory.
- [x] #121 reviewed against Phases 3 and 4 and recorded on the issue without
  a closing keyword: superseded and completed for rowing, still open for the
  SkiErg and BikeErg venues.

Found and left alone: the SkiErg and BikeErg venues' instanced groups still
go through `applyInstanceGroup` and are absent in the gate's ski capture
(#121 stays open for them); GPU time was not measured; the web venue GLBs
still load for rowing, wholly hidden.

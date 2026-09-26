# Blender Phase 2 - shell and oars

Owner-approved scope: rebuild the single scull and its sculling oars inside the
existing build-time contract. Keep the 7.8 m shell, the oarlock pivots, and the
node names and roles that build.rs and ReplayScene.qml map materials onto. The
rig tests and the hand-grip and oar-pivot checks pass unchanged. Leave the
SkiErg and BikeErg equipment, the athlete, the camera and the loop mapping
alone. File anything else as an issue.

- [x] Amend ADR 0016's source rule (the opening commit): procedural assets keep
  reviewed generation scripts; modelled assets may use a reviewed `.blend` plus
  a deterministic export and validation script.
- [x] Classify the shell and oars: procedural. The script is the source and no
  `.blend` is committed. The reasons are in the PR description.
- [x] Generate `rowing-shell.glb` (`tools/blender/shell.py`, `make blender-shell`):
  - hull section, taper and deck; canvases; the cockpit and gunwales;
  - three-stay riggers; round swivel pins;
  - sliding seat and tracks; foot stretcher with heel cups; fin; bow ball and
    breakwater;
  - oars: tapered shaft, grip, handle cap, button, sleeve, hatchet blade.
  Generic design only: no brand shapes or logos.
- [x] Hold it to the V3 rowing contract at build time
  (`rowing_shell::validate_rowing_shell`: names, roles, composite rules, the
  60,000-triangle budget as drawn). Pin the pins, grip and length to the Rust
  constants in tests.
- [x] Enforce the budget in the generator too, with containment (every cockpit
  part inside the hull at every seat position) and a fin below the water at the
  top of the bob.
- [x] Fit the seat, floor and stretcher to the athlete the app draws: skin the V4
  athlete with the replay's frames over a stroke and validate the skinning
  against a capture. Measure clearance against V3.
- [x] Wire the pack into the scene: `RowingRig` instances, the V3 rowing nodes
  hidden, the left blade reflected with a front-face-culled material copy.
- [x] Finish the look in the app's materials. Clearcoat stays on paint, carbon
  and blade. The hull's wet band comes from its vertex-colour masks, with no
  textures and no UV set.
- [x] Evidence: Blender contact sheet (front, side, top, three-quarter) and
  close-ups; Qt before/after at Low through Ultra in light and dark (same
  renderer, scheme, tier and moment); stroke-cycle close-ups; budget report;
  byte-identical rebuild; native Metal check on the Apple M5.
- [x] File what was found out of scope: #129 (the blades never square) and #130
  (the seat anchor and pelvis target sit below the web's).

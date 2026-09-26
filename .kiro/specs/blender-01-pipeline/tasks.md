# Blender Phase 1 - Direction C pipeline

Owner-approved scope: overcast championship morning and blue-hour companion,
same runtime frame truth, existing athlete retained. No new straight course.

- [x] Inspect current branch, references, Qt and Blender versions.
- [x] Implement seeded metre-scale generation, plain GLB export, budgets and
  Blender contact sheets. Preserve plain Git and record provenance.
- [x] Generate original daylight/blue-hour HDR skies and Qt-prefiltered probes.
- [x] Integrate stationary instanced buoys, water normals and hull materials.
- [x] Record sky policy in ADR 0016.
- [x] Complete Linux same-renderer baseline/light/dark/tier comparisons and pixel checks.
- [x] Verify unchanged runtime camera/boat/pose/oars and forward motion sequence.
- [x] Measure request-to-settled-row capture before/after; distinguish it from
  the first-presented-frame counter. Record single-run limitations.
- [x] Verify all seven assets byte-identical across independent generation
  after canonicalising opaque triangle order; importer probe passes for the
  features used. Optional extensions remain unverified and prohibited inputs.
- [x] Run fmt, workspace clippy, Qt-free/app tests and full light/dark gates;
  inspect the pixel-validated Qt replay stills and motion sequence on Linux.
- [x] Source-first closeout: reject unsafe GLB layouts and unexpected KTX metadata;
  confirm real instancing and compare isolated filter timings against main.
- [x] Obtain macOS/Metal visual verification on Apple M5 (light/dark, all tiers).
- [x] Verify interactive replay and performance on Apple M5 hardware iGPU.
- [x] Prepare one Phase 1 draft PR with actual Qt and Blender evidence.
- [x] Integrate current main: preserve qtbridge 0.3.0/MSRV 1.88/native controls,
  retain native styles as ADR 0015 and renumber Blender lighting to ADR 0016.
- [x] Recheck fixed-state parity, native Metal light/dark/tier visuals, instancing,
  short interactive playback, fmt/clippy/build and Qt-free/full app tests.

Phase 2 remains the shell/oar rebuild inside the existing 7.8 m rig contract.
Replacement anatomy/rigging is explicitly out of scope.

Before shell/oar work, refine ADR 0016's broad script-only source wording:
procedural assets use reviewed generation scripts; artistically modeled assets
use reviewed `.blend` sources plus deterministic export/validation scripts.
This follow-up is not a Python CAD framework or work included in Phase 1.

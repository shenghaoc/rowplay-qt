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
- [ ] Obtain macOS/Metal visual verification (this session is Linux).
- [ ] Verify interactive replay and performance on a hardware iGPU.
- [x] Prepare one Phase 1 draft PR with actual Qt and Blender evidence.

Phase 2 remains the shell/oar rebuild inside the existing 7.8 m rig contract.
Replacement anatomy/rigging is explicitly out of scope.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Blender Phase 3: environment and water — evidence

These are images for the Blender Phase 3 pull request (branch
`blender/03-environment-water`). They live on this image branch and are never
merged into `main`. All of them use the demo library only.

The Qt captures come from the Apple M5 (macOS 27.0, Qt 6.11.2, Cocoa/Metal).
"Before" is the Phase 2 parent, built in its own worktree, and "after" is
Phase 3; both sides show the same replay states, checked frame by frame.

- `tools/blender/capture.py` took the approved moment (demo 1001 at 208.829 s,
  mid-drive) at every tier, plus its 3 s motion sequence.
- Scratch probes of the same kind took the lap (eight loop positions at
  Medium) and a 12 s sequence (every 0.5 s from the approved moment).

The Qt images:

- `qt-medium-{light,dark}-before-after.jpg`: Medium, before on top and after
  below.
- `qt-tiers-{light,dark}-before-after.jpg`: Low, Medium, High and Ultra rows,
  before on the left and after on the right.
- `qt-lap-{light,dark}-before-after.jpg`: the chase view at 0, 125, … 875 m of
  the loop.
- `qt-environment-crop.jpg`: the environment band at native pixels, light and
  blue hour.
- `qt-water-before-after.jpg`: near and mid-distance water at native pixels.
- `qt-high-tower-shadow.jpg`: the retained finish tower's shadow on the new
  quay at High, with its twin, the shadow switched off.
- `qt-motion-before-after.gif` and `.mp4`: the 12 s sequence at 4 frames per
  second (2x), before on the left.
- `qt-motion-strip.jpg`: five of its frames, 3 s apart.

The Blender images are review renders (`tools/blender/review_environment.py`),
not acceptance evidence:

- `blender-overview.jpg`: Cycles, the approved chase view and a wider context
  view.
- `blender-light-and-blue-hour.jpg`: EEVEE, the chase view in both schemes.
- `blender-lap.jpg`: EEVEE, eight chase views round the loop.
- `blender-banks-and-water.jpg`: the woodland bank, the wetland, the campus
  quay, and a low view along the water.
- `blender-plan.jpg`: the plan view, fogged as Qt fogs it.
- `water-normal-16m-before-after.jpg`: 16 m of each water normal, shaded from
  a grazing light, seen from above. On the left, Phase 2's 4 m tile repeats as
  a lattice; on the right, Phase 3's 8 m tile.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Blender Phase 4: course dressing and venue furniture — evidence

These are images for the Blender Phase 4 pull request (branch
`blender/04-course-dressing`). They live on this image branch and are never
merged into `main`. All of them use the demo library only.

The Qt captures come from the Apple M5 (macOS 27.0, Qt 6.11.2, Cocoa/Metal).
"Before" is the Phase 3 parent (`blender/03-environment-water`), built in
its own worktree, and "after" is Phase 4; both sides show the same replay
states, checked frame by frame (51 states per scheme, plus the 25 states
of the motion sequence).

- `tools/blender/capture.py`'s set (the approved moment, demo 1001 at
  208.829 s, mid-drive, at every tier, plus its 3 s motion sequence) and,
  from a scratch probe of the same kind, the lap (eight loop positions at
  Medium), the zones (launch, finish, bridge, far bank, wetland; Medium,
  High and Ultra), the island with a ghost, and a 12 s sequence (every
  0.5 s from the approved moment).

The Qt images:

- `qt-medium-{light,dark}-before-after.jpg`: Medium, before on top and after
  below.
- `qt-tiers-{light,dark}-before-after.jpg`: Low, Medium, High and Ultra rows,
  before on the left and after on the right.
- `qt-lap-{light,dark}-before-after.jpg`: the chase view at 0, 125, … 875 m of
  the loop.
- `qt-launch-zone-before-after.jpg`, `qt-finish-zone-before-after.jpg`,
  `qt-bridge-before-after.jpg`, `qt-farbank-before-after.jpg`,
  `qt-wetland-before-after.jpg`: the zones at Medium, before on the left.
- `qt-island-before-after.jpg`: the island with a ghost, Medium and High.
- `qt-island-ghost-high-after.jpg`: the island with a ghost at High, after,
  at 1600 px.
- `qt-tiers-zones-after.jpg`: the launch, finish, bridge and wetland zones
  at Low, Medium, High and Ultra, after only.
- `qt-high-shadow-pair.jpg`: the shadow check's pair at High (the tower, the
  pontoon and the jetty cast) and its mask.
- `qt-motion-before-after.gif` and `.mp4`: the 12 s sequence at 4 frames per
  second (2x), before on the left.
- `qt-motion-strip.jpg`: five of its frames, 3 s apart.

The Blender images are review renders (`tools/blender/review_environment.py`,
EEVEE), not acceptance evidence:

- `blender-overview.jpg`: the approved chase view and a wider context view.
- `blender-zones.jpg`: chase views into the launch, finish, bridge, far-bank,
  coaching-pontoon and wetland zones.
- `blender-structures.jpg`: close-ups of the tower, the jetty, the launch
  pontoon, the campus, the bridge, the coaching pontoon, the boardwalk, the
  hide, a distance board and the island.
- `blender-light-and-blue-hour.jpg`: the chase view in both schemes.
- `blender-plan.jpg`: the plan view.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Blender Phase 2: shell and oars — evidence

These are images for the Blender Phase 2 pull request (branch
`blender/02-shell-oars`). They live on this image branch and are never merged
into `main`. All of them use the demo library only.

The Qt captures come from the Apple M5 (macOS 27.0, Qt 6.11.2, Cocoa/Metal):

- `tools/blender/capture.py` took the before (Phase 1 head) and after sets:
  demo 1001 at 208.829 s, the four tiers, and light and blue hour.
- The gate walk took the phase close-ups, with `ROWPLAY_PHASE_SHOTS=1` and
  `ROWPLAY_PHASE_CLOSEUPS=1`.

The images:

- `shell-contact-sheet.jpg`: Blender top, side, front and three-quarter views
  (Cycles), posed as the app clones the oars. The left blade is reflected.
- `shell-close-ups.jpg`: the cockpit, oarlock, blade and handle.
- `qt-medium-{light,dark}-before-after.jpg`: Medium, the boat region, before
  on top and after below.
- `qt-tiers-{light,dark}-before-after.jpg`: Low, Medium, High and Ultra rows,
  before on the left and after on the right.
- `stroke-close-ups-light-before-after.jpg`: the catch, mid-drive, finish and
  mid-recovery close-ups, before on the left and after on the right.
- `stroke-close-ups-dark-after.jpg`: the same four phases in blue hour.
- `wet-band-probe.jpg`: magenta marks the pixels that change (delta > 1)
  when the hull's vertex-colour masks are switched off (High, light).
- `blades-recovery-before-after.jpg`: both blades in two recovery frames.
  From left: before left, before right, after left, after right.

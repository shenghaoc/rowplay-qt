# Blender Phase 1 evidence

Never merge this image-hosting branch into main. These are demo-only review
artifacts for `blender/01-pipeline`, following `ui/screenshots`' convention.

Qt 6.11.2, Linux Wayland/OpenGL, Mesa llvmpipe. All before/after pairs use
the same renderer, demo workout 1001, elapsed 208.829 s, Medium, mid-drive.
The compared runtime pose/grip/tier records are identical except the sequence
counter. Original venue geometry, athlete, camera and loop mapping are retained.

- `light-before-after.png`, `dark-before-after.png`: original Qt left,
  Direction C / blue-hour Qt right.
- `light-motion.mp4`, `dark-motion.mp4`: original left, new right; 13 actual
  Qt renders at quarter-second replay intervals, encoded at 4 fps. The
  workout runs around 4.8 m/s; no accelerated orbit or texture scrolling.
- `motion-strip.png`: new Qt light at 0, 1.5 and 3 seconds.
- `buoy-contact-sheet.png`: four Blender views of the actual generated GLB.
- `c-reference-comparison.png`: Qt left, approved Blender C light right.
- `blue-hour-reference-comparison.png`: original too-dark Blender frame left,
  independently lit Qt blue-hour companion right. The old dark frame is not
  an accepted final target.

Alpha/pixel checks and frame equality are recorded in the implementation's
`docs/blender-audit.md`. Mac/Metal is not verified by these images.

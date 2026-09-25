# ui/screenshots — image hosting for the UI stack's PR bodies and issues

This orphan branch holds only the before/after screenshots and evidence crops
that the `ui/01-fixes` … `ui/07-docs` pull requests and their issues embed,
(`replay-fixes/`) the before/after captures of the pull request for
#84 and #85, and (`round2/`) the evidence for the design system's second
round (paused-replay rendering, the shadow check, the High-tier shadows and
the UI pull requests that follow), and (`round3/`) the third round's
evidence (the ghost's clock, the High-tier shadow fix, and the switch to
Qt's native styles).
It shares no history with `main` and is **never to be merged**: the pairs
belong in PR bodies, not in the repository's tree (the stack keeps one
current screenshot set under `docs/screenshots/`).

All captures are the demo library only (no Concept2 account data). They are
the runtime-error gate's own captures or xdotool-driven screen captures under
Xvfb + Mesa llvmpipe on Linux, and, from the Codex review round of #68 and
#70 on, captures made on macOS: the gate's captures, crops of the live
window, and scratch probes of the real controls, each named as such in the
PR body or issue that embeds it. Files are quantised or JPEG-compressed to
keep the branch small. Deleting the branch after the stack lands would break the
images in those PR bodies and issues.

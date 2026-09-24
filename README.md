# ui/screenshots — image hosting for the UI stack's PR bodies and issues

This orphan branch holds only the before/after screenshots and evidence crops
that the `ui/01-fixes` … `ui/07-docs` pull requests and their issues embed,
and (`replay-fixes/`) the before/after captures of the pull request for
#84 and #85.
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

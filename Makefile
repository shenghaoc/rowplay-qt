# SPDX-License-Identifier: GPL-3.0-or-later
BLENDER ?= blender

.PHONY: blender-assets blender-shell
blender-assets:
	PYTHONHASHSEED=0 "$(BLENDER)" -b --factory-startup --python-exit-code 1 -P tools/blender/build_all.py

# The shell and oars alone (Phase 2): the other authored assets and their pins stay as they are.
blender-shell:
	PYTHONHASHSEED=0 "$(BLENDER)" -b --factory-startup --python-exit-code 1 -P tools/blender/build_all.py -- --only shell

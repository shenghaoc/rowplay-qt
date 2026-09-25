# SPDX-License-Identifier: GPL-3.0-or-later
BLENDER ?= blender

.PHONY: blender-assets
blender-assets:
	PYTHONHASHSEED=0 "$(BLENDER)" -b --factory-startup --python-exit-code 1 -P tools/blender/build_all.py

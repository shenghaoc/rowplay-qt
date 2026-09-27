#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Supplemental native lower-body anatomy views, with equipment hidden.

This is explicitly outside the fixed-equipment A/B experiment. Uses its same
capture/restore implementation and clock; neutral clay reveals skin occluded
by the shell in normal replay. No geometry or transform changes.
"""
import hashlib
import json
from pathlib import Path
import sys
import capture_athlete as capture
from capture_contact import SAMPLES


def lower_cases(smoke=False):
    if smoke: raise ValueError('lower anatomy capture requires all phases')
    cases=[];step=300;count=0;scene='detailColumn.children[3]'
    def add(action):
        nonlocal step
        cases.append(f'case {step}: {action}; break');step+=1
    for workout,samples in SAMPLES.items():
        add(f'Replay.loadWorkout({workout}); Replay.loadGhost(-1); Replay.setQualityIndex(1)')
        for name,cycle in samples:
            if name=='ski-plant':continue
            add(f'Replay.setGuardCycleStep({cycle}); {scene}.auditMaterials("A"); {scene}.contactMask(true); {scene}.auditMaterials("B"); {scene}.auditView("lower"); root.grabSettledScene("{name}-lower-isolated")')
            add(f'{scene}.contactMask(false)');count+=1
    add('Qt.quit()')
    return cases,count


if __name__=='__main__':
    capture.capture_cases=lower_cases
    capture.main()
    out=Path(sys.argv[sys.argv.index('--output')+1])
    manifest=json.loads((out/'manifest.json').read_text())
    manifest.update(mode='supplemental isolated lower anatomy; equipment hidden, clay material',
                    views=['lower'],variants=['B'],
                    supplemental_instrument_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    (out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')

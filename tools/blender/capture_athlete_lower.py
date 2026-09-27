#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Supplemental native lower-body anatomy views, with equipment hidden.

This is explicitly outside the fixed-equipment A/B experiment. Uses its same
capture/restore implementation and clock; neutral clay reveals skin occluded
by the shell in normal replay. No geometry or transform changes.
"""
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
    capture.main(supplemental=True)

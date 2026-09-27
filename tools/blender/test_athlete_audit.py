# SPDX-License-Identifier: GPL-3.0-or-later
"""Independent analytic checks for Phase 5.2 measurement extensions."""
import unittest
import numpy as np
from audit_athlete import Nearest, stretch, topology, triangle_metrics
from capture_athlete import capture_cases, surface_palettes


class AthleteAuditTests(unittest.TestCase):
    def test_nearest_matches_exhaustive_on_ties_and_random_cloud(self):
        rng=np.random.default_rng(52)
        p=rng.normal(size=(147,3)); q=np.vstack([rng.normal(size=(93,3)),p[:4]])
        ids,dist=Nearest(p).query(q)
        expected=np.linalg.norm(q[:,None]-p[None],axis=2)
        np.testing.assert_allclose(dist,expected.min(1),rtol=1e-14,atol=1e-14)
        np.testing.assert_allclose(dist,expected[np.arange(len(q)),ids])

    def test_triangle_jacobian_rigid_scaled_sheared_and_degenerate(self):
        p=np.array([[0.,0,0],[1,0,0],[0,1,0],[2,0,0]])
        tri=np.array([[0,1,2],[0,1,3]])
        for mapping in [np.eye(3),np.diag([2.,.5,1]),np.array([[1,.7,0],[0,1,0],[0,0,1]])]:
            q=p@mapping.T+[12,-7,2]
            small,large,valid=stretch(p,q,tri)
            sv=np.linalg.svd(mapping[:,:2],compute_uv=False)
            np.testing.assert_allclose([small[0],large[0]],sorted(sv),atol=1e-12)
            self.assertEqual(valid.tolist(),[True,False])

    def test_topology_welds_only_identical_positions_and_counts_boundaries(self):
        p=np.array([[0.,0,0],[1,0,0],[0,1,0],[0,0,0],[3,0,0],[4,0,0],[3,1,0]])
        t=np.array([[0,1,2],[3,2,1],[4,5,6]])
        report,*_=topology(p,t)
        self.assertEqual(report['seam_duplicate_vertices'],1)
        self.assertEqual(len(report['components']),2)
        self.assertEqual(report['duplicate_triangles'],1)
        self.assertEqual(report['boundary_edges'],3)
        self.assertEqual(report['nonmanifold_edges'],0)

    def test_equilateral_quality_and_area(self):
        p=np.array([[0.,0,0],[1,0,0],[.5,np.sqrt(3)/2,0]])
        area,edges,aspect=triangle_metrics(p,np.array([[0,1,2]]))
        np.testing.assert_allclose(area,np.sqrt(3)/4)
        np.testing.assert_allclose(edges,1)
        np.testing.assert_allclose(aspect,1)

    def test_cases_cover_eight_required_phases_and_control(self):
        cases,count=capture_cases()
        text='\n'.join(cases)
        self.assertEqual(text.count('grabSettledScene('),count)
        for phase in ['row-catch','row-middrive','row-finish','row-halfslide','ski-approach','ski-pull','ski-release','ski-recovery','bike-top','bike-quarter']:
            self.assertIn(phase+'-chase-A',text)
        for phase in ['row-catch','ski-release']:
            for view in ['full','face','torso','left','right']:
                for variant in ['A','B','C','D','L','mask']:
                    self.assertIn(f'{phase}-{view}-{variant}',text)

    @unittest.skipUnless(__import__('pathlib').Path('reference/rowplay/src/lib/replay/renderer3dV4Assets.ts').exists(), 'pinned reference checkout required')
    def test_canonical_palette_has_all_eight_roles(self):
        self.assertEqual([r for r,c in surface_palettes()],['skin','jersey','lower','footwear','hair','trim','eye','face-detail'])



class EvidenceControlTests(unittest.TestCase):
    def test_frame_validator_rejects_a_pose_or_light_change(self):
        from publish_athlete_audit import validate_frames
        import copy
        frames=[]; experiments=[]
        fields=['frame','nodes','equipment','mirrored','poleGrips','rig','camera','fov','viewport','grip','contacts','tier','sport']
        for i in range(10):
            for variant in ['A','B','C','D','L']:
                name=f'pose-{i}-{variant}'
                frames.append(dict(name=name,**{key:[1] for key in fields}))
                experiments.append(dict(name=name,probe='fixed',probeExposure=1,keyBrightness=0 if variant=='L' else 1))
        self.assertEqual(len(validate_frames(frames,experiments)),10)
        bad=copy.deepcopy(frames);bad[1]['nodes']=[2]
        with self.assertRaisesRegex(ValueError,'changed nodes'): validate_frames(bad,experiments)
        bad=copy.deepcopy(experiments);bad[2]['keyBrightness']=2
        with self.assertRaisesRegex(ValueError,'lighting'): validate_frames(frames,bad)

class SupplementalCaptureTests(unittest.TestCase):
    def test_equipment_hidden_only_in_separate_lower_views(self):
        from capture_athlete_lower import lower_cases
        cases,count=lower_cases()
        text='\n'.join(cases)
        self.assertEqual(count,10)
        self.assertEqual(text.count('contactMask(true)'),10)
        self.assertEqual(text.count('contactMask(false)'),10)
        self.assertEqual(text.count('grabSettledScene('),10)
        with self.assertRaises(ValueError): lower_cases(smoke=True)


if __name__=='__main__': unittest.main()

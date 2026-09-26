# SPDX-License-Identifier: GPL-3.0-or-later
"""Instrument checks; run with Python containing numpy and Pillow."""
import unittest
import tempfile
from pathlib import Path
from PIL import Image
import numpy as np
from contact_skin import matrix, transform, surface_distance, sampled_surface, native_view


class ContactInstrumentTests(unittest.TestCase):
    def test_native_view_uses_the_held_root_not_a_racing_secondary_grab(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)
            held=Image.new('RGBA',(4,5),'red')
            held.paste(Image.new('RGBA',(4,3),'green'),(0,2))
            held.save(output/'sample.png')
            Image.new('RGBA',(4,3),'blue').save(output/'sample-view.png')
            view=native_view({'name':'sample','viewport':[4,3]},output)
            self.assertEqual(view.size,(4,3))
            self.assertEqual(view.getpixel((0,0)),(0,128,0,255))

    def test_triangle_interiors_edges_and_degenerate_edges(self):
        triangles = np.array([[[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]]])
        points = np.array([[.5, .5, 3.], [1., -2., 0.], [2., 2., 0.]])
        np.testing.assert_allclose(surface_distance(points, triangles), [3., 2., np.sqrt(2)])
        line = np.array([[[0., 0., 0.], [2., 0., 0.], [2., 0., 0.]]])
        np.testing.assert_allclose(surface_distance(points[:2], line), [np.sqrt(9.25), 2.])

    def test_pruning_keeps_a_large_triangle_with_a_distant_centroid(self):
        triangles = np.array([[[0., 0., 0.], [100., 0., 0.], [0., 100., 0.]],
                              [[0., 0., 1.], [1., 0., 1.], [0., 1., 1.]]])
        np.testing.assert_allclose(surface_distance(np.array([[.1, .1, .01]]), triangles), [.01])

    def test_native_basis_reconstructs_affine_rotation_scale_translation(self):
        basis = [[10, 20, 30], [10, 22, 30], [7, 20, 30], [10, 20, 34]]
        np.testing.assert_allclose(transform(np.array([[1, 2, 3]]), matrix(basis)), [[4, 22, 42]])

    def test_surface_sampling_includes_vertices_and_bounds_spacing(self):
        vertices = np.array([[0., 0., 0.], [.003, 0., 0.], [0., .004, 0.]])
        points, edges, _ = sampled_surface(vertices, np.array([[0, 1, 2]]), np.ones(3, dtype=bool))
        self.assertAlmostEqual(edges.max(), .005)
        for vertex in vertices:
            self.assertLess(np.linalg.norm(points-vertex, axis=1).min(), 1e-12)
        # A non-grid point inside this face is covered within the stated bound.
        self.assertLess(np.linalg.norm(points-[.0011, .0009, 0], axis=1).min(), .001)


if __name__ == '__main__':
    unittest.main()

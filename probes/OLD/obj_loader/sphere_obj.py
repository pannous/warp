#!/usr/bin/env python3
"""A UV sphere as Wavefront .obj text with about `faces` quads: sphere_obj.py <faces> > sphere.obj"""
import math, sys

faces = int(sys.argv[1])
rings = max(2, int(math.sqrt(faces / 2)))
segments = 2 * rings
print("# uv sphere")
for ring in range(rings + 1):
	theta = math.pi * ring / rings
	for segment in range(segments):
		phi = 2 * math.pi * segment / segments
		print(f"v {math.sin(theta) * math.cos(phi):.5f} {math.cos(theta):.5f} {math.sin(theta) * math.sin(phi):.5f}")
corner = lambda ring, segment: ring * segments + segment % segments + 1
for ring in range(rings):
	for segment in range(segments):
		print(f"f {corner(ring, segment)} {corner(ring, segment + 1)} {corner(ring + 1, segment + 1)} {corner(ring + 1, segment)}")

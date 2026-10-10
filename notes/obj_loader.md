# The .obj loader of samples/manual_3d_object_viewer.warp (card mv-would, 2026-10-10)

User: "Would it make sense to create a library with an optimized object loader, or is our own implementation optimized?"
Answer: no library for now; the warp loader (`triangles(obj)`, 15 lines) is linear and fast enough, and the viewer's
cost is its shader, not the loading.

Measured (probes/obj_loader: sphere_obj.py makes UV spheres, loader_as_written.warp is the sample's loader,
reference.rs a plain native parser = the core of a library like tobj; release warp 1.2.5, M-series, load average 30,
user time minus the 0.09 s of an empty program):

| faces (quads) | triangles | file   | warp loader | native reference |
|---------------|-----------|--------|-------------|------------------|
| 10^3          | 1936      | 45 KB  | ~0.01 s     | 0.2 ms           |
| 10^4          | 19600     | 485 KB | ~0.06 s     | 3 ms             |
| 10^5          | 198916    | 5.3 MB | ~0.7 s      | 17 ms            |

- Linear: `points = points + [...]` no longer copies the list each time. Native is ~40× faster, but a 5 MB model loads
  in under a second, and the sample's house has 14 faces.
- The real limit for big models is the viewer: its fragment shader tests every triangle for every pixel
  (256² pixels × 200k triangles a frame is far beyond any GPU at 30 fps). Big models need rasterizing (a vertex
  shader drawing the triangles with a depth buffer: paint/gpu_render has no vertex stage yet) before a faster loader
  matters.
- A library would pay off for the other OBJ features (normals `vn`, texture coordinates `vt`, materials `mtllib`,
  negative indices), not for speed: then a warp `use obj` module (lib/obj.warp) first, an FFI parser only if
  measurements of real models demand it.

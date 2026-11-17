# Meshlets

Library `lib/meshlets.warp`, sample `samples/graphics/meshlet_grid.warp`, tests `test/graphics/test_meshlets.pairs.warp`.
The sample is not named `meshlets.warp`: a file of that name is what `use meshlets` loads, so the sample would import itself.

This is a CPU picture of clustered triangles. It is not Unreal Nanite, and it is not a port of three.js.

## Reference

- Post: https://x.com/dangreenheck/status/2116283720806105244 (52 UTC). The wording "like Nanite",
  the "<24 hours", and "1B triangles at 150 FPS" are the poster's. They were not remeasured.
- three.js PR 35191, merge `e4d0c9eb716d77813ea9b56467a7d96a1285d67d`, 2026-10-29 13:48:40 UTC, MIT,
  Copyright © 2010-2026 three.js authors.
- Clustering and simplification in that PR call meshoptimizer 1.1 (MIT, Copyright (C) 2016-2026 Arseny Kapoulkine),
  shipped there as wasm. None of that wasm, and no meshoptimizer source, is in this tree.
- The cone rule used here is the published one: a cluster is backfacing when
  `dot(center - camera, axis) > cutoff * distance + radius`, `cutoff = sin(half-angle)`, and a cone wider than
  meshoptimizer's 0.1 minimum dot is not used. The comparison is strict, so a sphere that only touches the cone
  or the frustum plane stays visible. The bounding sphere is the sphere of the axis-aligned box, which contains
  every vertex and is larger than the smallest sphere.

What that three.js commit actually does, and what this library does not copy:

- It builds up to six whole-mesh LOD levels with meshoptimizer, then meshlets of 64 triangles.
- LOD is one level per object, from projected error. Clusters of that level are then frustum, cone, and
  occlusion culled. Occlusion is a depth pyramid of the previous frame.
- `VisibilityBufferDrawingPipeline` rasterizes small triangles in a compute shader. The class comment calls it
  experimental. There is no cluster DAG, no streaming, and no residency control.

## What warp does here

Input for a coarser mesh is a regular heightfield: `cells_x` and `cells_y` even and at least 2, one height per
vertex, row-major. Odd counts and a short height list are refused.

- Fine triangles: two per cell, counter-clockwise from +z.
- Clusters: consecutive triangles, at most 64. This is not a spatial meshlet build.
- Coarse mesh: the same grid with step 2, so every other vertex remains. Triangle count drops by 4.
- Error: the largest vertical distance from a fine vertex to that coarse triangulation. A flat grid's error is 0.
  A 2 by 2 grid whose center height is 1, and whose other heights are 0, has error 1. This is not a simplifier
  for an arbitrary triangle soup.
- Per object, perspective error is `error / distance * cot(half fov) * screen_height / 2`. The last level within
  the pixel threshold is chosen. Level errors must not decrease. Orthographic projection is not implemented.
- Frustum planes are `nx, ny, nz, d` with inside `n·p + d >= 0`. Cone culling runs only when every triangle has
  area and the normals fit the cone. A degenerate triangle, or two triangles that face opposite ways, disables
  the cone, including when the camera is behind the cluster.

## GPU path

`paint` / `gpu_render` still draw a full-image triangle, or instanced quads when the shader has its own vertex
stage and `values.indices` is absent. Two opt-ins, the same in `lib/host/native_host/gpu.warp` and
`web/playground/host-gpu.js`:

- `values.indices`, a flat list of uint32 vertex ids, is an index buffer. The draw is indexed triangles. The
  vertex stage pulls attributes from storage: `values.vertices[vertex_index]` is the vertex the index names.
  A shader with no `@vertex` function is refused. With a depth target the indexed draw is opaque; without one
  it blends like a quad.
- `values.depth`, a number other than 0, attaches `depth32float`. The pass clears it to 1 and keeps that
  texture for the next frame that asks. Compare is less-equal, so a fragment on the far plane stays, and the
  test writes depth. A quad draw keeps its blend; discarded fragments do not write depth.

`grid_indexed` is the heightfield as that vertex list and index list. `mesh_draw_indices` flattens triangle
records `[a, b, c, part]` to the index buffer. `frame_rate(frames, spent_ms)` is the number the samples print
on stderr when an argument is `fps` (CPU work; a paint is included only when the frame would paint anyway).

Three more opt-ins, again the same on both hosts:

- `values.compute`, shader text with `@compute fn main`, runs in the same encoder before the render pass.
  It may bind the value arrays, plus `indices` and `indirect`, by the names in its own `@binding`s. One shader
  binds at most 8 storage arrays. The value map may hold 12, because the compute pass and the draw bind their
  own subsets. `u32` and `i32` arrays are only `indices` and `indirect`; every other array is float bytes.
- `values.workgroups` is one to three positive ints. The default is one workgroup.
- `values.indirect` is four uint32s for `drawIndirect`, or five for `drawIndexedIndirect` (that one needs
  `indices`). The compute shader writes them. The cone test is meshoptimizer's (MIT, Arseny Kapoulkine,
  https://github.com/zeux/meshoptimizer). The indirect-buffer shape follows three.js PR 35191 (MIT,
  https://github.com/mrdoob/three.js/pull/35191). The WGSL in `lib/meshlets.warp` is written here.

`INDEX_CULL` copies the kept clusters' indices and sets an indexed indirect draw. `INSTANCE_CULL` compacts
`draws` and `spans` in place and sets the instance count to the sprites plus the kept clusters times
`MESHLET_TRIANGLES`. A cutoff of 1 skips the cone test.

`view[1]` is LOD parameters: cot(half fov), screen height, threshold, and how many planes follow.
Planes start at `view[2]`. `LOD_INDEX` picks the last level whose projected error is within the threshold,
the same rule as `lod_picked` / `chosen_level`, then culls that level. `LOD_INSTANCE` does that per tree
from `lods`: trees first (first level, level count, local distance), then each level (first cluster,
cluster count, error). The printed counts stay the CPU's choice. The picture uses the GPU's.

After a depth pass the host keeps one max-depth per 8 by 8 tile. The next frame's cull reads that
pyramid as `pyramid`: a cluster is dropped when its nearest depth is past every tile it covers.
The first frame at a size is all far, so nothing is occluded. `values.previous` is still the previous
color frame.

Still absent:
- A paint uniform is at most 16384 floats (64 KB).
- There is no compute rasterizer, no atomic visibility buffer, and no resolve pass.
- There is no mesh shader or task shader.
- There is no geometry paging or streaming. The three.js reference does not have that either.
- The words above are the existing paint and gpu_compute. A further host word would go through `core/native` and the emitter.

## Tree meshes

`samples/graphics/ez_trees/*.eztree` are seeded trees from Dan Greenheck's EZ Tree (MIT, commit
`dcf309bd86bd521083d9c70f01f2de45fdc7c457`). The notice beside them is the licence. The exporter is
`scripts/ez_tree/export.mjs`. No EZ Tree texture, demo asset, or EZ Tree Pro file is in this tree.

A file is little-endian. `EZT1`, then a u32 level count. Each level is u32 vertex count, u32 triangle count,
f32 error, then that many vertices `(x, y, z, u, v)` and triangles `(u16 a, b, c, part)`. `part` 0 is bark and
1 is a leaf. The tree is normalized: the trunk stays at the xz origin, the base is y = 0, and the height is 1.
`read_tree_mesh` / `tree_mesh_of` build one `TreeMesh`. The fine level's stored error is 0, the same contract as
a flat grid, so a near camera can select it. A coarser level's error is at least the measured one-sided
Hausdorff distance of that level and of every finer level, and at least 0.0001, so the errors do not decrease.
`chosen_level` would otherwise keep the last level whenever two errors were equal.

Clusters are still consecutive groups of 64 triangles. That is not a spatial partition: a cluster can hold
bark and leaves, and none of the oak's fine clusters has a cone tight enough to cull. Frustum culling still
drops a cluster whose sphere is outside. `local_view(planes, camera, place)` moves the planes and the camera
into one tree. `place` is `x, z, yaw, scale`: world point = `(x, 0, z) + scale * R(yaw) * p`.

`pack_tree_meshes` lays every level into one vertex list `(x, y, z, u)`, one texel list `(v)`, and one triangle
list of absolute indices plus the part. A level's `base` is its first triangle there, and a cluster's `at` is
its first triangle in the level.

## Sample value

`samples/graphics/meshlet_grid.warp` returns, in order: flat fine triangles, flat coarse triangles, the two flat
cluster sizes, the peak's coarse error, peak triangles kept at the fine level and at the coarse level, flat
cluster counts in front, behind, and outside one plane, then the oak: level count, fine triangles, coarse
triangles, fine clusters, coarse clusters, the level at distance 2 and at distance 40 (one pixel, cot 1,
height 135), fine clusters kept by the plane `x >= 0.05`, and how many of those fine clusters can be cone-culled:
`[96, 24, 64, 32, 1, 8, 2, 2, 0, 0, 2, 1998, 1146, 32, 18, 1, 2, 30, 0]`. The fifth value is the peak error, 1.
The oak's fine error is 0 and its coarse error is about 0.0529.

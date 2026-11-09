# Meshlets

Library `lib/meshlets.warp`, sample `samples/graphics/meshlet_grid.warp`, tests `test/graphics/test_meshlets.pairs.warp`.
The sample is not named `meshlets.warp`: a file of that name is what `use meshlets` loads, so the sample would import itself.
Card key intended: `nanite-library-poc` (title Warp Nanite). The `todo` CLI was not on the machine that wrote this
(`todo: command not found`, exit 127), so the board was not updated.

This is a CPU picture of clustered triangles. It is not Unreal Nanite, and it is not a port of three.js.

## Reference

- Post: https://x.com/dangreenheck/status/2116283720806105244 (2026-10-30 21:38:52 UTC). The wording "like Nanite",
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

## GPU parity that this does not have

Doing what `MeshletDrawingPipeline` does on the GPU would need host support warp does not have. None of it was
added. `lib/host/native_host/gpu.warp` and `web/playground/host-gpu.js` stay as they are (the shared GPU path).

- `paint` / `gpu_render` draw a full-image triangle, or instanced quads when the shader has its own vertex stage.
  There is no indexed triangle mesh and no vertex-pull of a meshlet.
- There is no indirect draw. The three.js pipeline writes draw counts from a compute pass into an indirect buffer.
- There is no depth attachment and no depth pyramid. `values.previous` is the previous color frame, not a
  hierarchical depth.
- `gpu_compute` binds at most 8 storage arrays and returns them to the CPU. The meshlet cull uses more buffers
  (instances, bounds, cones, queues, counters, visibility, indirect arguments) and then draws without reading
  the indices back.
- A paint uniform is at most 16384 floats (64 KB).
- There is no compute rasterizer, no atomic visibility buffer, and no resolve pass.
- There is no mesh shader or task shader.
- There is no geometry paging or streaming. The three.js reference does not have that either.
- A new host word for any of the above would go through `core/native` and the emitter. That is out of scope.

## Sample value

`samples/graphics/meshlet_grid.warp` returns, in order: flat fine triangles, flat coarse triangles, the two flat
cluster sizes, the peak's coarse error, peak triangles kept at the fine level and at the coarse level, and flat
cluster counts in front, behind, and outside one plane:
`[96, 24, 64, 32, 1, 8, 2, 2, 0, 0]`. The fifth value is the peak error, 1.

# Aquarium

Fragment-based aquarium approximation. Not a faithful WebGPU port. No GPU frame has been shown.

The picture is `samples/gpu/aquarium.warp`. The reef is that file; the fish mesh and its swim are `samples/gpu/fish_mesh.warp`, shared with `samples/gpu/fish.warp`. There is no `lib/aquarium.warp`. `test/aquarium/test_aquarium.warp` imports the sample file, which brings the functions and not the `on frame` paint.

## Reference

[greggman/webgpu-aquarium](https://github.com/greggman/webgpu-aquarium) commit `055da77885a571e85970b2bb3807b0b504ce1a1c` ("Mention Size README.md"). MIT, Copyright (c) 2026 Gregg Tavares. The full upstream `LICENSE.md` at that revision is reproduced below. No mesh or texture files are copied. The screenshots in that repo stay there.

The formulas kept are the tiled travelling cosines and the Jacobian caustic (`src/shaders/waves.wgsl`, `src/render/caustics.ts`, alpha 5.5, fringe 0.94 / 1 / 1.06, compress `pow(·, 1.25) * 0.55`), the tropical absorption `[0.1, 0.034, 0.02]`, scattering `0.016`, sun and ambient (`src/world/world.ts`), and `sunAtDepth` / `ambientAtDepth` / the Henyey–Greenstein phase / view transmittance (`src/shaders/water.wgsl`). Schools use that sim's roaming home and the hashed slot in the leader's heading frame (`src/sim/fish.ts`). Followers are drawn on the slot the compute shader springs toward. The slot hash is a float hash; theirs is the uint `hashU`.

The fish are not that sim's ellipsoid school. They are the Fish Kit's low red sea bream, `samples/gpu/madai.fish` (408 vertices, 722 triangles, a colour a vertex), CC0, author なちゃっと (@nachat_dayo). The terms, the URLs and the bake are `samples/gpu/madai.NOTICE` and `notes/fish.md`. Eight of them swim the reef at different scales (0.55 to 3.8 times the half-metre mesh, so about 0.25 m to 1.7 m long), on the three school paths, each with its own phase and pace. A frame poses the mesh four times, a quarter of the tail-beat apart, and every fish wears one of those poses. The ray is transformed into that fish's space (place, heading, scale). A bounding sphere skips a fish whose sphere the ray misses, or that sits farther than a hit already found. A pixel that misses every sphere tests eight spheres and no triangles. One that hits a fish tests that fish's triangles, and any nearer sphere the ray entered first. About 650 of the 722 triangles survive the cutout. Four poses are those triangles once a frame (about 8,000 corners, a few hundred kilobytes), not eight copies of the mesh.

## Upstream license

Exact `LICENSE.md` from greggman/webgpu-aquarium at `055da77885a571e85970b2bb3807b0b504ce1a1c` (sha256 `22bf9b52b86f92408391fc504d696bc1c1604a789f7502ac59ebcbf32901379f`). Kept in this note. No separate license file was added. The header of `samples/gpu/aquarium.warp` names that revision.

```
MIT License

Copyright (c) 2026 Gregg Tavares

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## What paints

`paint(aquarium_color, view_width, view_height)` is one fragment shader, the same path as `samples/gpu/sdf_scene.warp` and `samples/gpu/ocean.warp`. The sample is

```bash
warp samples/gpu/aquarium.warp
```

With a window, `on frame` keeps swimming. Without one (`--no-window`), that frame runs once and the paint is a still (decision on-frame-headless). `view_color(x, y, t)` is the same picture at one pixel, which the tests call.

## Not this slice

Their reef is many compute and render passes (meshes, 10–20k fish, shadows, volumetrics, post effects); a faithful port needs a library call that dispatches a WGSL compute pass into a storage buffer and binds it on a later pass, in the shared GPU host (`lib/host/native_host/gpu.warp`, `web/playground/host-gpu.js`).

## Tests

`warp test test/aquarium` checks waves, caustics, absorption, the school, the instance sphere and one pixel at two times. `test/aquarium/test_madai.warp` checks that the eight scales differ, and that a ray through a moved, turned, scaled fish hits where the single posed mesh does, for one triangle and for the baked bream. None of this claims a GPU frame was shown. A headless run of the sample is a still from `paint`, and without a GPU that still is the CPU `paint_each`, not a fragment shader that ran.

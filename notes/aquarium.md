# Aquarium

A paintable reef in `lib/aquarium.warp`, shown by `samples/gpu/aquarium.warp`.

## Reference

[greggman/webgpu-aquarium](https://github.com/greggman/webgpu-aquarium) commit `055da77885a571e85970b2bb3807b0b504ce1a1c` (2026-10-01, "Mention Size README.md"). MIT, Copyright (c) 2026 Gregg Tavares, `LICENSE.md`. No mesh or texture files are copied. The screenshots in that repo stay there.

The formulas kept are the tiled travelling cosines and the Jacobian caustic (`src/shaders/waves.wgsl`, `src/render/caustics.ts`, alpha 5.5, fringe 0.94 / 1 / 1.06, compress `pow(·, 1.25) * 0.55`), the tropical absorption `[0.1, 0.034, 0.02]`, scattering `0.016`, sun and ambient (`src/world/world.ts`), and `sunAtDepth` / `ambientAtDepth` / the Henyey–Greenstein phase / view transmittance (`src/shaders/water.wgsl`). Schools use that sim's roaming home and the hashed slot in the leader's heading frame (`src/sim/fish.ts`). Followers are drawn on the slot the compute shader springs toward. The slot hash is a float hash; theirs is the uint `hashU`.

## What paints

`paint(aquarium_color, view_width, view_height)` is one fragment shader, the same path as `samples/gpu/sdf_scene.warp` and `samples/gpu/ocean.warp`. The sample is

```bash
warp samples/gpu/aquarium.warp
```

With a window, `on frame` keeps swimming. Without one (`WARP_NO_WINDOW`), that frame runs once and the paint is a still (decision on-frame-headless). `view_color(x, y, t)` is the same picture at one pixel, which the tests call.

## Not this slice

Their reef is many compute and render passes: procedural meshes, 10–20k integrated fish, indirect draws, a caustic texture, terrain, shadow maps, volumetrics, SSAO, TAA, depth of field, bloom. `paint` has none of those targets. Tone mapping here is Reinhard and gamma, not their present-pass grade. Fog uses `fog_scale = 0.45` in place of `frame.misc.z`. Eight waves, not twelve. The seabed is a ripple, not their heightfield or voxels.

A faithful multi-pass port needs a library call that dispatches a WGSL compute pass into a storage buffer and binds that buffer on a later pass. That lives in the shared GPU host (`lib/host/native_host/gpu.warp`, `web/playground/host-gpu.js`), which this card does not edit. The fragment picture does not need it.

## Tests

`warp test test/aquarium` checks waves, caustics, absorption, the school, the body distance and one pixel at two times. It does not claim a GPU frame was shown.

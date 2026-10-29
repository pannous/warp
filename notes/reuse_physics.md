# Reusing a physics engine (card reuse-physics, user policy reuse-giants 2026-10-29)

What we build ourselves today: nothing general. samples/advanced/particles.warp integrates gravity and bounces
by hand (Euler steps, wall checks); there is no rigid body, collision shape, joint or broad phase anywhere in lib/.
So this review picks what a `use physics` library would bind, before anyone starts writing a solver in warp.

## Verdicts

| Need | Take | Binding | License | Same in browser and native |
|---|---|---|---|---|
| 2D rigid bodies, shapes (circle, capsule, polygon), joints, contacts, sensors, ray casts | **Box2D v3** (Erin Catto, C17, 2024 rewrite) | C ABI with plain value handles (`b2BodyId`, `b2ShapeId` are small structs of ints), so `from 'box2d'` lines in a lib/physics.warp work without a wrapper except for the few calls taking definition structs (`b2DefaultBodyDef`), which one thin C file flattens into int and float arguments | MIT | yes: Box2D builds to wasm32 with clang/emscripten as a plain module (box2d3-wasm exists), so the playground loads the same engine |
| 3D rigid bodies, characters, vehicles | **Jolt Physics** (C++, used by Horizon Forbidden West; JoltC is its C API) | C ABI through JoltC | MIT | yes: JoltPhysics.js is its wasm build |
| 2D and 3D in one engine, deterministic across platforms | **Rapier** (Rust; rapier2d / rapier3d) | as a plain wasm module of flat exports (its JS bindings are wasm-bindgen; a C-ABI or WIT layer is ours to write) | Apache-2.0 | yes, by design: the one engine whose cross-platform determinism is a feature (`enhanced-determinism`), good for replays and lockstep networking |

Take Box2D v3 first: games and simulations in warp's samples are 2D, its C API maps to warp's `from 'c'` lines
almost one to one, it is the smallest (about 20 k lines of C) and the same code compiles for the playground. Jolt
when 3D comes (with reuse-graphics' wgpu-native renderer); Rapier only if determinism across machines becomes a
requirement, since it needs a binding layer of our own either way.

## Not taken
- **Chipmunk2D** (MIT, C): simpler and older than Box2D v3, slower stacking, little maintenance.
- **Bullet** (zlib, C++): 3D, mature, but its C API is minimal and its development moved on; Jolt is the modern
  choice with a real C API.
- **PhysX 5** (BSD-3, C++): heavy, GPU parts CUDA-only, no C API.
- **matter.js / planck.js**: JavaScript only, for the playground alone; Box2D-as-wasm covers both lanes.

## We stop building
- Any hand-written solver beyond teaching samples: particles.warp stays as a sample of plain warp, but bodies,
  collisions, stacking and joints come from the engine.
- Collision queries (ray casts, overlaps, point tests) for games: Box2D's `b2World_CastRay` / `b2World_OverlapAABB`.

## Shape
lib/physics.warp: `world(gravity)`, `body(world, x, y, shape: circle 0.5)`, `step(world, 1/60)`, `position(body)`,
each a warp word over the C calls; a frame loop of `use draw` reads positions and paints them. Native: libbox2d
beside libwarp_wasmtime, resolved like the sound and window wrappers. Browser: the same C built to a plain wasm module
the playground loads. Open core question, shared with reuse-sound and reuse-regex: an optional native library needs
a call-time failure instead of a load-time one, and a library's wasm module needs the playground's loader.

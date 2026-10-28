# Runtime linking: the compiler knows its runtime by heart

Roadmap card: runtime-linking

Decision runtime-prepended (user): as in wasp, the runtime is built separately so the compiler stays small and clean.
A compile reads no runtime or library sources: it reads the prebuilt wasm modules (runtime.wasm, one per library
module, beside the compiler; not embedded, user), and a program is the runtime with the user's code appended behind it. Build artifacts stay standalone.

## Shape
- The runtime is emitted once, at `warp rebuild`: its functions with every call operand padded to a fixed width
  (wasp: nops behind the call), so an index can be rewritten in place without moving code; its types are the one
  canonical type section at fixed indices. It is written as runtime.wasm beside warp.wasm; the compiler reads it (exports, name section, its call-site list) per compile.
- Emitting a program: the runtime's functions come first and keep their indices; the user's functions are numbered
  from the runtime's count on. Only the user's indices are computed, the runtime's need no fixing.
- Everything else that shifts indices is laid out the same way: the runtime's imports, globals, data (texts) and
  element segments (closures) first at fixed places, the program's behind them. Host imports a program adds would
  shift function indices: the runtime declares the full host import list up front, or its padded calls are rewritten
  by the shift.
- Library modules (sound, units, markup…) are prebuilt the same way and appended between runtime and user code, each
  rewritten by its base offset through its padded calls (`use sound; midi(440Hz)` costs 2 s today, all of it compiling
  sound.warp).
- What isn't wasm and needs it (sugar rules, alias and keyword tables, operator instances) is pre-baked, e.g. as
  constant GC globals of warp.wasm.

## Steps (sub-cards of runtime-linking)
| card | step |
|---|---|
| runtime-prebuilt | runtime.wasm emitted at rebuild beside warp.wasm, padded calls + canonical types + call-site list |
| runtime-prepended | the emitter writes the runtime first and the user code behind it; no runtime words compiled per program |
| modules-prebuilt | bundled modules prebuilt and appended by base offset; WARP_LIB overrides compiled from source |
| tables-as-constants | sugar rules, alias/keyword tables, operator instances as constant GC globals |
| shaking-build-options | `warp build` options for trimming: none, Binaryen metadce/wasm-opt, own linker later |
| runtime-separate-modules | runtime, modules and program linked by import/export (browser, AOT combining) |
| runtime-shaking | our own warp linker (Later) |

## Removing the unused runtime (decided: all options open, user)
All options stay open and are exposed to the build process (`warp build` options); none is the only way.
- Higher priority: existing tools (Binaryen's wasm-metadce / wasm-opt) for trimmed artifacts (card
  shaking-build-options); separate modules linked by import/export, in the browser and wherever the ahead-of-time
  compile combines modules natively, so size may not matter at all (card runtime-separate-modules; GC types match
  across modules through the canonical type section).
- Low priority: our own clean linker in warp (card runtime-shaking, Later): reachability from the user's code over the
  baked call-site list, compact the function list, rewrite the padded operands; the same for globals, data and
  elements. From first principles, not a port of wasp's C++ linker.

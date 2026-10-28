# Runtime linking: the compiler knows its runtime by heart

Roadmap card: runtime-linking

Decision runtime-prepended (user): as in wasp, the runtime is built separately so the compiler stays small and clean.
A compile reads no runtime or library sources and no per-run file: the finished runtime is part of warp.wasm, and a
program is the runtime with the user's code appended behind it. Build artifacts stay standalone.

## Shape
- The runtime is emitted once, at `warp rebuild`: its functions with every call operand padded to a fixed width
  (wasp: nops behind the call), so an index can be rewritten in place without moving code; its types are the one
  canonical type section at fixed indices. Its bytes are embedded in warp.wasm (no file beside it).
- Emitting a program: the runtime's functions come first and keep their indices; the user's functions are numbered
  from the runtime's count on. Only the user's indices are computed, the runtime's need no fixing.
- Everything else that shifts indices is laid out the same way: the runtime's imports, globals, data (texts) and
  element segments (closures) first at fixed places, the program's behind them. Host imports a program adds would
  shift function indices: the runtime declares the full host import list up front, or its padded calls are rewritten
  by the shift.
- Library modules (sound, units, markup…) are prebuilt the same way and appended between runtime and user code, each
  rewritten by its base offset through its padded calls (`use sound; midi(440Hz)` costs 2 s today, all of it compiling
  sound.warp).
- What isn't wasm (sugar rules, alias and keyword tables, operator instances) is baked as constant GC globals of
  warp.wasm: ready at instantiation, nothing decoded.

## Steps (sub-cards of runtime-linking)
| card | step |
|---|---|
| runtime-prebuilt | runtime emitted at rebuild with padded calls + canonical types, embedded in warp.wasm |
| runtime-prepended | the emitter writes the runtime first and the user code behind it; no runtime words compiled per program |
| modules-prebuilt | bundled modules prebuilt and appended by base offset; WARP_LIB overrides compiled from source |
| tables-as-constants | sugar rules, alias/keyword tables, operator instances as constant GC globals |
| runtime-shaking | dead-code removal: drop runtime functions the program never reaches (see below) |

## Removing the unused runtime (open, to discuss)
The padded calls make the call graph readable without decoding instructions (a list of call sites per function), so
dropping unreached functions is: reachability from the user's code, compact the function list, rewrite each padded
operand. Options:
1. A clean linker in warp, from first principles, replacing wasp's C++ one: reachability + compaction + rewriting for
   functions, globals, data and elements, one pass, its call-site list baked with the runtime.
2. Shake only for `warp build` artifacts; `warp`/`warp pure` runs keep the whole runtime, whose machine code is cached
   once, so a run pays nothing for its size.
3. Binaryen's wasm-metadce / wasm-opt for release builds: no linker of our own, an external tool.
4. During development the runtime as its own module that programs import (GC types match across modules by the
   canonical type section), merged and shaken only for standalone builds.

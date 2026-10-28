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
- What isn't wasm and needs it (sugar rules, alias and keyword tables, operator instances) is pre-baked into warp.wasm
  as packed literals unpacked lazily on first use (Supervisor's call: constant GC globals would build every tree at
  instantiation, even for hello).

## runtime.wasm format (card runtime-prebuilt; draft, owner: fixer)
One file serves both prepending (runtime-prepended) and import/export linking (runtime-separate-modules). It is an
ordinary, valid wasm module: every runtime word compiled once, from its declared kinds (`any` is a node), with every
made_case kind made, since no program decides any more what it makes.
- **Types**: the fixed GC types (emitter.warp gc_types, in gc_names order) at indices 0…G-1, each its own singleton
  group, so isorecursive canonicalization makes them identical in any module that declares the same list in the same
  order; one function, `canonical_type_entries()`, writes that list for runtime.wasm and for a program module. Then
  the runtime's function types. A program's class structs (one rec group) and its function types come after them.
- **Imports**: the full host-word list, declared up front in a fixed order (module and field names as today), so a
  program's host words never shift the runtime. A program's own C imports (FFI) go after them and shift the runtime's
  function indices: those are rewritten through the call-site list below.
- **Functions**: the runtime words in a fixed order, then the hooks. Every `call` and `ref.func` operand is a padded
  5-byte LEB128 (a valid non-minimal encoding), so an index changes in place without moving code.
- **Hooks**: the functions whose code depends on the program (today's made_late: closure·call·N for each arity,
  closure_text, shown_big_int, instance_set, instance_entries, instance_field_at, instance_place, is_class_instance,
  the text·shown dispatcher). In runtime.wasm each is a stub with its final signature. Prepending replaces a stub's
  body with the program's code; a separately linked program fills the exported funcref table `hooks` at its start, and
  the stubs call through it (open: a stub that calls through the table, or two stub flavours).
- **Globals**: the runtime's (runtime/walks.warp and the others) first, at fixed indices; a program's after them.
- **Data**: one passive segment, the runtime's texts from offset 0. A prepended program's texts are appended behind
  them (text_offset continues), reusing a runtime text that is already there.
- **Tag**: the error tag, index 0, exported.
- **Exports**: memory as `memory`, every runtime function and global under its warp name, the tag, the table `hooks`.
- **Custom sections**:
  - `name`, as the emitter writes it today (functions, locals, types, fields, globals).
  - `warp.runtime`, the layout: G, the import count, the function count, the global count, the data size, the
    hooks' indices, and each runtime word's parameter and result ValueKinds. The emitter's call_values needs these;
    a wasm type alone can't tell Bool, Int and Char apart.
  - `warp.calls`, the call-site list: the byte offset of every padded call and ref.func operand in the code section.

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

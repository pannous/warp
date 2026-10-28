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
- What isn't wasm and needs it is pre-baked (card tables-as-constants, done): every bundled module's parse is packed
  (pack) into the compiler beside its text, `LIBRARY_PARSES = library_parses()` in core/parser.warp, which uses.warp's
  module_text fills in when the compiler is compiled; `source_parse(path, source)` unpacks it on first use when the
  text is the bundled one (a WARP_LIB edit is parsed). Sugar rules, the alias table and the operator modules read it;
  no os.kept for them. Packed text rather than constant GC globals: globals are built eagerly at instantiation and
  would need the class layouts; a packed text is one data segment, unpacked lazily. The Rule, Operator and Spellings
  instances are derived from the baked trees per run (O(rows)). Measured (2026-10-28, hello-sized program, fresh
  module cache): cold 7.1 s → 0.22 s, warm 0.55 s → 0.22 s; the cold 7 s were the sugar-rule parses. A compiler built
  by a seed that bakes none parses at run time (the plain def library_parses), so the seed is refreshed after landing.

## runtime.wasm format (card runtime-prebuilt, owner: fixer; built by core/emitter/prebuilt.warp)
`warp rebuild` writes it beside warp.wasm (compiler.warp's command `runtime [file]`); validated by wasm-tools.
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
  the text·shown dispatcher). In runtime.wasm each is a stub with its final signature: its parameters, `i32.const slot`,
  `call_indirect` of its own type on table 0. Prepending replaces a stub's body with the program's code; a separately
  linked program fills the exported funcref table `hooks` at its start (closure·call for arities 0…hook_arities, 8).
- **Left out**: a word outside the core on its own (an enum's method naming the program's enum: Separator·text) is not
  in runtime.wasm; warp.runtime names it, a program compiles it as today.
- **Globals**: the runtime's (runtime/walks.warp and the others) first, at fixed indices; a program's after them.
- **Data**: one passive segment, the runtime's texts from offset 0. A prepended program's texts are appended behind
  them (text_offset continues), reusing a runtime text that is already there.
- **Tag**: the error tag, index 0, exported.
- **Exports**: memory as `memory`, every runtime function and global under its warp name, the tag, the table `hooks`.
- **Custom sections**:
  - `name`, as the emitter writes it today (functions, locals, types, fields, globals).
  - `warp.runtime`, the layout as warp-notation text: `types` (G), `imports`, `functions`, `globals`, `data` (bytes),
    `hooks` (names in table-slot order), `left`, `words: {"name": [index, [parameter kinds], result kind]}`. The
    emitter's call_values needs the kinds; a wasm type alone can't tell Bool, Int and Char apart.
  - `warp.relocations` (replaced warp.calls): a vector of [offset, kind, space] uleb triples, one per padded 5-byte
    operand of the code: offset its byte in the code section's content (behind its vector count); kind what it indexes,
    0 a function (call), 1 a type (a GC op's type immediates), 2 a global (global.get/set), 3 a text's data offset
    (i32.const ahead of array.new_data); space the index space: 0 runtime, 1 own, 2+k the k-th module a module's
    declarations name. runtime.wasm: all space 0 (5665 entries). Global initializers name only the fixed GC types and
    stay unpadded. One reader and one rule (emitter/module_layout.warp): `relocations_of(wasm)`, and
    `rebased(code, relocations, shifts)` adding shifts[space][kind] to each operand in place, for a module appended
    behind others and for the runtime's functions behind a program's C imports.

## Library modules (card modules-prebuilt; owner: tables-as-constants)
- `<module>.wasm` beside the compiler, of runtime.wasm's shape: the canonical types, padded operands, `warp.runtime`
  (the module's own layout: function, global and data counts, each exported word's kinds) and `warp.relocations`.
  Its calls into the runtime hold the runtime's fixed indices; its own and other modules' are numbered from 0 and
  rebased by the module's base offset when it is appended between the runtime and the program.
- `warp.declarations`: what a program needs of the module (signatures, classes, globals) as one packed node (pack),
  so a compile reads no module source. A module under WARP_LIB that differs from the bundled one is compiled from
  source.
- Modules have no hooks of their own: a module's class (units) is a class of the linked program, its text() and ==
  go into the program's dispatcher; module code calls the runtime's hook stubs.

## Hooks (runtime words calling program code: a class's text(), ==)
One hook list in the warp.runtime section. Prepended: the stub bodies are replaced by the program's. Separately linked:
the stubs call through an exported funcref table the program fills at start. One runtime build serves both.

## Separate modules (card runtime-separate-modules; draft, owner: separate-modules)
The same runtime.wasm, not copied into the program but instantiated once and imported, as a page or an AOT build
combines modules natively. Nothing is rewritten: a program's runtime words are imports, and imports come first in
the function index space anyway, so the program numbers only its own functions, as today.
- **Program module** (`warp build --separate`, emitter flag `module.links_runtime`): the canonical types first
  (canonical_type_entries(), so its node, text and list types are the runtime's), then its class structs. Every
  runtime word it calls is a function import `warp_runtime.<word>` with the word's type (instead of compiling the
  word); runtime globals it reads are global imports; memory, the error tag and the `hooks` table are imported too.
  Its texts stay in its own passive segment (array.new_data is per module). Its start code fills `hooks` with
  ref.func of its hook bodies (table.set by the index the warp.runtime section names).
- **Library modules** the same way: each imports `warp_runtime`, exports its words; a program imports them as
  `warp_module.<module>.<word>`, so `use sound` costs an instantiate, not a compile.
- **Hosts**: wasmtime (src/run): instantiate runtime.wasm once per store (from its .cwasm), then the program with a
  Linker that resolves `warp_runtime` to that instance; the browser (web/playground worker.js) the same with
  `WebAssembly.instantiate(program, { warp_runtime: runtime.exports, warp_host })`.
- **Trimming** the shared runtime against one program's imports is where wasm-metadce fits (roots: the imported
  names); a runtime shared by several programs stays whole.
- **Check**: hello and a class with text() (a hook) built separately print the same as built prepended.

## Steps (sub-cards of runtime-linking)
| card | step |
|---|---|
| runtime-prebuilt | runtime.wasm emitted at rebuild beside warp.wasm, padded calls + canonical types + call-site list |
| runtime-prepended | the emitter writes the runtime first and the user code behind it; no runtime words compiled per program |
| modules-prebuilt | bundled modules prebuilt and appended by base offset; WARP_LIB overrides compiled from source |
| tables-as-constants | sugar rules, alias/keyword tables, operator modules: their parses baked as packed texts (done) |
| shaking-build-options | `warp build` options for trimming: none, Binaryen metadce/wasm-opt, own linker later |
| runtime-separate-modules | runtime, modules and program linked by import/export (browser, AOT combining) |
| runtime-shaking | our own warp linker (Later) |

## Removing the unused runtime (decided: all options open, user)
All options stay open and are exposed to the build process (`warp build` options); none is the only way.
- Higher priority: existing tools (Binaryen's wasm-metadce / wasm-opt) for trimmed artifacts (card
  shaking-build-options); separate modules linked by import/export, in the browser and wherever the ahead-of-time
  compile combines modules natively, so size may not matter at all (card runtime-separate-modules; GC types match
  across modules through the canonical type section).
- `warp build --trim=<way>` (src/cli/trimming.rs; also `warp self build`): `optimize` (the default, and bare
  `--trim`), `unused`, `none`, `warp` (refused until runtime-shaking). Trimmed by default (user, 2026-10-28), the way
  picked by measurement: wasm-opt with wasmtime's feature list (never `-all`: custom descriptors write exact
  references wasmtime refuses) and the name section kept. Measured 2026-10-28 on the seed-era modules:

  | module | compiled | `unused` (--remove-unused-module-elements) | `optimize` (-O2) |
  |---|---|---|---|
  | hello | 2135 | 1562, 33 ms | 917, 32 ms |
  | sudoku | 32694 | 31086, 25 ms | 19640, 59 ms |
  | raytracer | 48280 | 46126, 26 ms | 31431, 87 ms |
  | neural_net | 49098 | 46647, 26 ms | 29474, 72 ms |
  | snake | 86064 | 82755, 28 ms | 61172, 82 ms |

  -O2 is about a third smaller for 30–60 ms more, so it is the default; outputs identical. Today's emitter already
  writes only the reached runtime words; once the runtime is prepended whole (runtime-prepended), `unused` matters.
- Low priority: our own clean linker in warp (card runtime-shaking, Later): reachability from the user's code over the
  baked call-site list, compact the function list, rewrite the padded operands; the same for globals, data and
  elements. From first principles, not a port of wasp's C++ linker.

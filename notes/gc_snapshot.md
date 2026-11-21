# GC snapshot: a warm wap compiler without 50k constant initializers

**Question (user, 2026-11-21):** wap wants a warm compiler, with its prepared state already built: the parsed types and
prelude, the rule tables and the symbol maps, about 50k GC objects. Baking them as wasm GC constant initializers costs
~270 B of machine code each and 73 s of compile time at 50k objects. Can a patched wasmtime dump a running instance's
GC heap and reload it? What do others do? How cheap is the in-language alternative?

**Recommendation:** a compact passive data segment plus a decoder written in wap (option C). It is portable to every
host, its machine code does not grow with the number of objects, and it builds 50k objects in about 4 ms. A wasmtime
heap-snapshot patch (option A) is feasible, about 300 lines, but it would save only those ~4 ms. It would also tie the
warm state to the native host and to one wasmtime build. Keep it parked as a later optimisation.

## Measurements (wasmtime 49.0.2 CLI, M-series Mac, 2026-11-21)
The bench is in the scratchpad: gen.py writes either the constant-initializer form or the decoder form of N nodes.
Each node is `struct {kind i32, name (ref array i8), next (ref null node)}`, so one node is two objects: the struct
and its name.

| form | N nodes | compile | .cwasm | build at run time |
|---|---|---|---|---|
| const globals (struct.new + array.new_fixed), copying | 2 000 | 0.9–2.8 s | 1.5 MB | – |
| const globals, copying | 10 000 | 11.5–24.7 s | 8.3 MB | – |
| const globals, drc | 10 000 | 4.7 s | 8.1 MB | – |
| const globals, single-pass regalloc | 10 000 | 2.5 s | 14.3 MB | – |
| data segment + decoder loop (array.new_data + struct.new) | 50 000 | 0.05 s | 1.1 MB (≈ the data) | ≈ 3–4 ms over the 2k baseline (~70 ns per node, both objects) |
| same | 500 000 | 0.05 s | – | ≈ 30 ms (drc), ≈ 64 ms (copying, GCs on the way) |

- The constant-initializer form costs about 800 B of machine code per node: the struct, its 4-byte name and the global.
  Compile time grows faster than linearly. Upstream, issue #14210 tracked this superlinear compile under the copying
  collector; it was closed as fixed 2026-09-14. #15382, open since 2026-11-19, reports quadratic regalloc memory: 4000
  globals take more than 4 GB, and wasm_of_ocaml's CI ran out of memory on it. The mitigations (drc,
  `regalloc-algorithm=single-pass`) only shrink the constant factor.
- The decoder's machine code is constant. The data costs roughly 5–16 B per object, depending on how tightly it is
  packed.

## A: patch wasmtime to dump and reload the GC heap
**What the internals allow.** Measured on warp's own patched wasmtime 50 (crates/wasmtime-capi/patched, which already
carries three patches, 433 lines):
- The GC heap is a `crate::vm::Memory`, a linear byte region, and a `VMGcRef` is a `NonZeroU32` offset into it
  (vm/gc/gc_ref.rs:131). Copying the bytes is therefore enough to move every intra-heap pointer, as long as the reload
  puts them at the same base.
- The copying collector (vm/gc/enabled/copying.rs, Cheney semi-space, bump allocation) leaves the live objects
  contiguous in `[active_space_start, bump_ptr)` after a GC. Each header holds its object size, so the heap can be
  walked linearly. Its `scan`/`scan_field` already rewrite every ref field of an object through trace infos, which is
  exactly the relocation pass that a load at a different base needs.
- Each header is `{kind: u32, ty: VMSharedTypeIndex}` (wasmtime-environ vmtypes.rs:811). `ty` is an engine-wide
  registry id, not a module type index, so the dump must translate it to module type indices and the reload must
  translate it back to the new engine's ids. This step is required: a new process registers types afresh.
- These are not position-free, so the prepared state must not contain them:
  - funcref fields: a `FuncRefTableId` into a table on the store;
  - externref: an id of host data;
  - exnref.
  wap's prepared state is structs, arrays and i31, so it qualifies. The dump should refuse anything else loudly.

**Patch outline (~250–350 Rust lines plus ~30 C lines in the native host):**
1. `GcHeap::image()` and `GcHeap::load_image(bytes, delta)` on the copying heap: copy the active semi-space out and
   back in, set `bump_ptr`, register the trace infos for every type present, and relocate ref fields when the bases
   differ (≈ 100 lines).
2. Store level: run a GC first, then take the image. Its roots are the instance's ref globals and tables, not a stack
   or user roots, so dump only between calls. Translate header `ty` to and from module type indices, and write the
   globals (≈ 120 lines).
3. C API `wasmtime_instance_gc_image_save/load` (≈ 40 lines), plus a versioned format: wasmtime version, collector,
   module hash.
4. In the native host's main.c: load the image beside the cached .cwasm (≈ 30 lines; the program cache keys by hash,
   commit df81f63).

**Risk.** It touches private structs (CopyingHeap, GcStore, the type registry), so every wasmtime upgrade means a
rebase. It works for the copying collector only. Only the native wap gains anything: the browser and any other host
still need a fallback, which means option C has to exist anyway. Wasmtime will not take it upstream soon: the only
related issue, #14615, is about serialising the heap for debugging, not for restoring it.

**Gain.** A memcpy of about 2 MB takes ~0.2 ms. The decoder takes ~4 ms, so A saves about 4 ms per compile.

**Variant without a patch:** a host-side walker over wasmtime's public Rust GC API (StructRef/ArrayRef field reads,
`StructRef::new`). This is rejected: at ~1 µs per call it is slower than the in-wasm decoder, and the C API does not
expose struct or array access anyway.

## B: what others do (sources in the research notes, 2026)
- Wizer is now `wasmtime wizer`. Its docs say "reference types are not supported yet", and it does not snapshot GC
  heaps.
- Binaryen's wasm-ctor-eval serialises GC data into `ctor-eval$global` defining globals: the same
  constant-initializer form, with the same blowup. It cannot emit `array.new_data`, because that is not a constant
  instruction in the GC MVP. No proposal for a constant `array.new_data` or for struct data segments exists.
- V8's startup snapshots do not cover wasm at all. No runtime (wasmtime, V8, WAMR, WasmEdge, wasmer) restores a
  wasm GC heap.
- Toolchains that do ship large static GC data at startup use data segments plus runtime construction, eagerly or
  lazily:
  - Kotlin/Wasm: strings come from `array.new_data`, built lazily and cached.
  - dart2wasm: large constants are lazy, behind a nullable global and an init function.
  - Hoot: a synthesised start function, split into partitions once it got huge.
  - TeaVM splits its start functions because V8 limits function size.
  - wasm_of_ocaml keeps constant globals and is fighting #15382 (PR #2570 inlines single-use globals).

## C: in-language: data segment + decoder in wap (recommended)
- **Producer:** the compiler that is being built runs its preparation, then packs the prepared graph into bytes. The
  next build emits those bytes as a passive data segment.
- **Consumer:** at startup, one loop rebuilds the graph:
  - `array.new_data` for texts and byte runs;
  - `struct.new` and `ref.i31` per object;
  - an array of refs for back-references.
- **Node packing:** almost all of the state is wap's uniform `Node` enum (wap/nodes.wap), so one generic node packer
  covers it. Precedent: warp's packed_nodes.warp packs and unpacks any node in 177 lines of warp, with varints and
  texts deduplicated by back-reference. warp's side files `modules/sugar_rules.packed` (106 KB) and `index.tsv`
  (98 KB, split by tab in ~1 ms) are that scheme kept on disk.
- **Placement:** in wap the bytes go inside the compiler module rather than in side files. There is then no file
  lookup, no staleness check and one artifact. The native host's program cache stores the compiled result.
- **Core cost:**
  - ~80–110 lines: pack plus unpack over Node, with texts deduplicated.
  - Plus the emitter's ability to place a passive data segment from a value (≈ 10 lines if not already there).
  - Hash maps (symbol maps, rule tables) are either rebuilt on load (re-hashing 50k keys is ~1–2 ms) or packed as
    their backing arrays. Rebuilding is simpler and keeps the format independent of the layout.
  - `Data(value: any)` payloads that are not Nodes need their own case or must stay out of the warm state.
- **Lazy variant:** unpack a definition's slice only when it is first reached, as warp does with signature slices.
  It pays only for what a program uses and suits hello-sized programs; decide this after the eager version is
  measured.
- **Risk:** low. Plain wasm, every host, any collector. The format is versioned by the compiler hash that the program
  cache already uses.

## Comparison
| option | core lines | other code | startup for 50k objects | compile cost | portable | risk |
|---|---|---|---|---|---|---|
| constant initializers | ~0 (emitter has them) | – | ~0 at run time | 73 s, ~40 MB machine code | yes | blowups upstream (#15382) |
| A: wasmtime heap image | 0 | ~300 Rust lines patch + ~30 C lines | ~0.2 ms | none | native only | rebase per wasmtime upgrade; private internals |
| C: data segment + decoder | ~80–110 (+~10 emitter) | – | ~4 ms (+~1–2 ms map rebuild) | none | yes | low |
| C lazy | +~20 | – | proportional to use | none | yes | low |

Build C now. Revisit A only if the ~4 ms ever shows up as a large share of a warm compile. It can be layered on later
as a cache of the decoder's result, using the same versioning key.

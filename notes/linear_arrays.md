# Linear arrays (user, 2026-10-06; card linear-memory)

User: "Create an explicit array type for linear data, but discourage its use by saying that the compiler will prefer
picking it automatically."

## Syntax and meaning
- `linear xs = int[n]`, `linear xs = float[n]`: n zeroed cells in the module's linear memory, one block
  `[count: i64][n cells of i64 or f64]`, 8-byte aligned, from the bump heap texts use (never freed, like texts).
- Declaring one shows the hint (educate_once, topic `linear-array`, "got it" silences it), naming its element type:
  `prefer xs = float[n] over linear xs = float[n]: the compiler picks where a list of numbers lives by itself, linear
  memory included; linear only forces it`. Not where linear arrays are paired by `dot` or `.*` (card linear-hint,
  pinned by tests/lists/test_linear_hint.rs).
- The compiler picks linear memory (card compiler-picks-dot, shared_arrays.rs picked_linear): a plain `xs = float[n]`
  assigned once, paired by `dot` or `.*` with another such float array, every other mention a cell read/write/add,
  `#xs`, `count(xs)`, `xs.count` or `for x in xs`, becomes `linear xs = float[n]`. dot of 10^6 then runs as
  linear_dotf (2–3 ms) instead of over GC lists (13 ms). Anything else (a reassignment, `print(xs)`, a call, an
  append, a GC list partner) keeps both as GC lists. samples/dot.warp has no `linear` any more.
- Reads like any list: `xs#i`, `xs#i = v`, `xs#i += v`, `#xs`, `count xs`, `xs.count`, `for x in xs {…}`, passing
  it to a function (by reference, as shared arrays); used as a whole value (printed, returned) it is the list of its
  cells. Outside 1..n: `index out of range`. A `go` cannot take one (an error says to use `shared`): another task has
  another memory.
- Int cells are i64 and wrap on overflow (no big integers in a cell), unlike the automatic typed arrays.

## Implementation
- src/lowering/shared_arrays.rs: the rewrite of `shared` arrays, with a storage kind (Host / Linear) choosing the words.
- src/wasm_emitter/linear_arrays.rs: `linear_new/count/get/set/add` and `getf/setf/addf`, the module's own functions;
  called like C functions (signatures in crate::ffi under the library `linear`, which import_manager never imports and
  effects counts as pure), so every context gets typed i64/f64 calls.

## Measured (probes/linear_array_bench.sh, 10^7 writes then 10^7 reads, CLI eval incl. compile)
- `linear xs = int[n]` 0.18–0.24 s, automatic `xs = int[n]` (wasm GC array) 0.19–0.23 s: the same speed. Linear memory
  pays for its layout, not speed: one block a host or GPU backend (P118, `@gpu` maps only) can take without copying.

## Later (linear-memory card)
- The compiler picking linear memory by itself above a threshold, when a backend needs the block (GPU, host SIMD).
- Freeing blocks (an arena per run or per call) once programs allocate many.

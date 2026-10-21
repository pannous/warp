# Linear arrays

An explicit array type for linear data, discouraged by a hint: the compiler picks linear memory by itself.

## Syntax and meaning
- `linear xs = int[n]`, `linear xs = float[n]`: n zeroed cells in the module's linear memory, one block
  `[count: i64][n cells of i64 or f64]`, 8-byte aligned, from the bump heap texts use (never freed, like texts).
- Declaring one shows the hint (educate_once, topic `linear-array`, "got it" silences it), naming its element type:
  `prefer xs = float[n] over linear xs = float[n]: the compiler picks where a list of numbers lives by itself, linear
  memory included; linear only forces it`. Not where they are paired in a way the compiler would not pick linear
  memory for (tests/lists/test_linear_hint.rs).
- The compiler picks linear memory (shared_arrays.rs picked_linear): a plain `xs = float[n]`
  assigned once, paired by `dot` or `.*` with another such float array, every other mention a cell read/write/add,
  `#xs`, `count(xs)`, `xs.count` or `for x in xs`, becomes `linear xs = float[n]`. dot of 10^6 then runs as
  linear_dotf (2–3 ms) instead of over GC lists (13 ms). Anything else (a reassignment, `print(xs)`, a call, an
  append, a GC list partner) keeps both as GC lists.
- Mapped by a numeric kernel: `xs.map(x => body)` / `map(xs, x => body)` whose
  lambda gpu_maps::kernel translates (arithmetic, sqrt, sin, max …, captured numbers) counts like a dot pairing, so a
  plain `xs = float[n]` becomes linear and the map runs as the f64x2 kernel or one loop into a new block. A lambda
  calling a user function keeps the GC list. probes/perf/auto_linear_map.warp (20 maps of 10^6 in a loop): 0.08 s.
- Reads like any list: `xs#i`, `xs#i = v`, `xs#i += v`, `#xs`, `count xs`, `xs.count`, `for x in xs {…}`, passing
  it to a function (by reference, as shared arrays; a parameter declared a list of its items, `block: floats`,
  `[float]`, `list of float` or `list`, takes it too, one of other items is refused); used as a whole value (printed, returned) it is the list of its
  cells. Outside 1..n: `index out of range`. A `go` cannot take one (an error says to use `shared`): another task has
  another memory.
- Int cells are i64 and wrap on overflow (no big integers in a cell), unlike the automatic typed arrays.

## Implementation
- src/lowering/collections/shared_arrays.rs: the rewrite of `shared` arrays, with a storage kind (Host / Linear) choosing the words.
- src/wasm_emitter/lists/linear_arrays.rs: `linear_new/count/get/set/add` and `getf/setf/addf`, the module's own functions;
  called like C functions (signatures in crate::ffi under the library `linear`, which import_manager never imports and
  effects counts as pure), so every context gets typed i64/f64 calls.

## Measured (probes/linear_array_bench.sh, 10^7 writes then 10^7 reads, CLI eval incl. compile)
- `linear xs = int[n]` 0.18–0.24 s, automatic `xs = int[n]` (wasm GC array) 0.19–0.23 s: the same speed. Linear memory
  pays for its layout, not speed: one block a host or GPU backend (P118, `@gpu` maps only) can take without copying.

## Later
- Freeing blocks (an arena per run or per call) once programs allocate many.

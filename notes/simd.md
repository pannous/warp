# SIMD for numeric maps

Roadmap card: fast-numbers (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).
GPU maps: notes/gpu.md. Linear arrays: notes/linear_arrays.md.

## Support
- wasm simd128 (fixed 128-bit: i64x2, f64x2, i32x4, f32x4) is on by default in wasmtime (Cranelift lowers it to NEON
  / SSE-AVX) and in every current browser (Chrome 91+, Firefox 89+, Safari 16.4+). Relaxed SIMD is not needed.
- simd128 loads and stores only from linear memory (`v128.load`); warp's int and float lists are GC arrays read with
  `array.get`. So SIMD needs the numbers in linear memory (`linear xs = float[n]`, or the automatic choice for float
  maps, card auto-linear-floats).
- Ints are unbounded (an overflow check per lane) and decimals exact (`i * 0.5` of an Int is a ratio); floats map
  one to one: f64x2 is IEEE as f64.

## Measured (probes/simd, y = x*0.5 + 1, ns per item)
| | wasmtime 8 MB | wasmtime 80 KB (in cache) | V8 8 MB | V8 80 KB |
|---|---|---|---|---|
| GC array, new result array | 2.1 | 2.1 | 2.7 | 2.5 |
| GC array, in place | 1.3 | 1.0 | 2.1 | 2.2 |
| GC → memory, f64x2, → new GC array | 2.6 | 2.7 | 8.7 | 4.9 |
| linear memory, scalar | 0.6 | 0.4 | 0.9 | 0.3 |
| linear memory, f64x2 | 0.4 | 0.17 | 0.9 | 0.26 |

- SIMD pays only with numbers in linear memory, and little when memory-bound; copying GC → memory → GC loses it all.
- Most of the win is the representation (a typed loop over memory), f64x2 itself adds 5-15%.
- A for loop's item counter `x·index` (only 0 and ++, below the count) needs no big-integer checks and reads with a
  direct `array.get` (big_int::bounded_counters, list_dispatch walking_counter).
- NaN canonicalization after every float operation doubled a float loop; it now happens only where a float's bits are
  observable (P119, floats::canonical_nan).
- Tried and dropped: a map preallocating its result with the source's length: zero-filling and the copying GC's work
  on one large array cost more than the doublings.

## f64x2 map kernels over linear float arrays
- `ys = xs.map(x => body)` / `map(xs, x => body)` with xs a linear float array and body float arithmetic of x and
  number literals (`+ - * /`, unary minus, `sqrt`, `abs`, groups like `(x + 1) * 2`) is the call
  `linear_mapf·x·<body>(xs)` (shared_arrays.rs float_map; ys becomes a linear float array too). The emitter
  (wasm_emitter/lists/linear_arrays.rs emit_float_map) reads the kernel back from its name: a new block, `v128.load`
  two cells, the body in f64x2, `v128.store`, an odd last cell in f64. Any other body (a captured variable, a call, a
  comparison) stays the general map.
- probes/simd/linear_map_bench.py: ~1.1 ns per item for the kernel against ~9 ns for the automatic GC `float[n]` with
  the general map. Each map takes fresh zeroed pages (blocks are never freed).
- `go xs.map(x => …)` with a float kernel splits on the host: parallel.rs kernel_map keeps the map
  (SPLIT_ATTRIBUTE); shared_arrays.rs split_mapped calls `task_split_mapf(kernel, xs)` from 2^18 cells up
  (SPLIT_FEWEST), and src/host/tasks/mod.rs split_map maps one slice per core, each in an instance of its own
  (exported `ffi_linear_mapf·…` and `ffi_linear_new`), copying cells in and out. 0 (in a task, in the browser, below
  the size) means the program calls the kernel itself. 10 rounds over 4M floats: 0.74 s against 2.12 s without go
  (tests/control/test_split_kernel.rs).

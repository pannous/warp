# SIMD (and GPU) for numeric maps: survey and measurements (async worker, 2026-10-06, card simd-map)

## Support
- wasm simd128 (fixed 128-bit: i64x2, f64x2, i32x4, f32x4) is standard: on by default in wasmtime 49 (Cranelift
  lowers it to NEON on this Mac / SSE-AVX on x86) and in every current browser (Chrome 91+, Firefox 89+, Safari 16.4+).
  Relaxed SIMD (fma, relaxed swizzle) is standard in Chrome, behind flags elsewhere; not needed for a first step.
- The catch for warp: simd128 loads and stores only from linear memory (`v128.load`). warp's int and float lists are
  GC arrays (`(array (mut f64))`, type_manager emit_typed_list_types), read element by element with `array.get`;
  there is no v128 access to a GC array. SIMD therefore needs the numbers in linear memory.
- Ints: warp Ints are unbounded (a fixnum overflows into a big integer handle), so `i64x2.add` would need an overflow
  check per lane; and decimals are exact (`i * 0.5` of an Int is a ratio). Floats map one to one: f64x2 is IEEE as f64.

## Measured (probes/simd, y = x*0.5 + 1, ns per item, 1e9 items)
| | wasmtime 8 MB | wasmtime 80 KB (in cache) | V8 (node 26) 8 MB | V8 80 KB |
|---|---|---|---|---|
| GC array, new result array | 2.1 | 2.1 | 2.7 | 2.5 |
| GC array, in place | 1.3 | 1.0 | 2.1 | 2.2 |
| GC → memory, f64x2, → new GC array | 2.6 | 2.7 | 8.7 | 4.9 |
| linear memory, scalar | 0.6 | 0.4 | 0.9 | 0.3 |
| linear memory, f64x2 | 0.4 | 0.17 | 0.9 | 0.26 |

And warp itself today (release build, 1M floats `xs = float[1000000]`): `xs.map(x => x * 0.5 + 1)` 15 ns per item,
`for x in xs { s += … }` 7 ns, appending `out.add(…)` another 6 ns.

## Reading
1. The big cost is not the missing SIMD: warp's map is 7x a tight GC loop (15 vs 2.1 ns): generic list reads,
   appends that grow, value semantics. A typed kernel (preallocated result array, `array.get`/`array.set`, the inlined
   numeric body) is the first and largest step, and it is what SIMD and GPU need anyway: a recognized pure numeric
   kernel over a float array.
2. SIMD pays only with numbers in linear memory: 2.5x over scalar memory in cache under wasmtime, little under V8 and
   nothing when memory-bound (8 MB). Copying GC → memory → GC loses everything (2.6 vs 2.1 ns).
3. So SIMD becomes worth it once float arrays (`float[n]`, `h*w` arrays) can live in linear memory: then 0.17-0.4 ns
   against today's 15, about 40-90x, most of it from the representation, the rest from f64x2.

## Where the time goes in warp's own loops (2026-10-06, after the sized-array fix)
`for x in xs { s += x * 0.5 + 1 }` over `float[1000000]`, 100 rounds, the compiled module run by wasmtime 49 CLI
(probes/simd/time_runs.py style), ns per item:

| module | plain | fuel | NaN canonicalization | fuel + NaN (warp's engine) |
|---|---|---|---|---|
| before (main) | 4.3 | 6.2 | 7.6 | 7.7 |
| loop counter proven an i32, items read with array.get | 3.8 | 3.9 | 7.3 | 7.2 |

- The step (wasm_emitter big_int::bounded_counters, list_dispatch walking_counter): a for loop's item counter
  `x·index` is only ever 0 and ++, below the count, so its compare and step need no big-integer checks and
  `x·items#(x·index+1)` is a direct `array.get`: about 10% plain, and the fuel cost of the loop is gone.
- The dominant cost is now `cranelift_nan_canonicalization` (warp-runtime engine.rs deterministic_config): +3.4 ns
  per item, doubling a float loop, and it applies to f64x2 lanes too. Canonicalizing only where a float's bits can
  be observed (print, text, comparison by bits, memory stores, host calls) instead of after every operation would keep
  the determinism decision and remove most of the cost: question to the Interviewer.

- Tried and dropped: a map preallocating its result with the source's length (`list_capacity(xs)`, array.new_default
  of the full size) instead of growing by push: 20 maps of 1M floats took 0.37-0.42 s against 0.31-0.33 s growing
  (wasmtime CLI, best of 5). Zero-filling and the copying GC's work on one large array cost more than the doublings.
  A map of 1M floats is now 14.5 ns per item (old: 19.5); the push itself (bounds, length store, growth) and GC
  allocation are what is left besides NaN canonicalization.

## Steps
1. (smallest, next) Typed map kernel: `xs.map(x => numeric body)` over a float array known statically as one emits
   `array.new_default(len)` + a loop of `array.get` / body / `array.set` (measured shape: 2.1 ns, 7x today).
2. Float arrays in linear memory (a `$floats` view: offset + length in a struct, the bytes in memory, freed by the
   GC finalizer-less arena per run): f64x2 kernels for step 1's maps; `go xs.map(f)` of such a kernel stays on one
   thread with SIMD below a size threshold, splits into tasks above it (shared arrays, P44, already live in the host).
3. GPU: see "GPU" below, a question for the user.

## GPU (note only, question sent to the Interviewer)
WebGPU in the browser and wgpu natively run WGSL compute shaders. A kernel from step 1 (pure, numeric, one float
array in and out) translates to WGSL almost line by line. Costs: a host dependency (wgpu, ~1-2 MB, Metal/Vulkan/DX12),
a copy to and from the GPU per map (only worth it beyond ~1e6 items or heavy bodies), f32 on most GPUs (f64 rare),
async readback in the browser (fits the playground's worker + Atomics.wait). Recommendation: not before steps 1-2;
then an explicit `@gpu` on the map (no silent offloading: results in f32 differ), native first through wgpu.

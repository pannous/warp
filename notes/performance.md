# Performance: what the benchmarks found (2026-10-05)

Benchmarks: `probes/night/bench_lists.sh [n]` (lists, closures, maps, text building), run with the release CLI
(`WARP=…/release/warp`). Keep n unknown at compile time when timing one idiom by hand (`n=2000000+random_below(1)`),
otherwise constant evaluation may hide the loop. Profile the generated module apart from warp:
`warp compile x.warp`, then `wasmtime run -W gc=y,function-references=y,exceptions=y,tail-call=y --profile=guest,out.json
--invoke main x.wasm` (Firefox profiler); `/usr/bin/sample <pid>` on the debug CLI shows the host's side.

## Fixed
| idiom | before | after | cause |
|---|---|---|---|
| map of 4000 text keys, write loop | 11.7 s | 0.49 s | DRC collector + empty GC heap + key comparison |
| same, write then read | 16.1 s | 0.59 s | map_entry_has_key made two symbols per entry |
| `t += "x"` 200000 times | out of memory | 0.09 s | every append copied the text into fresh linear memory |
| `"\(i)"` 200000 times | 0.40 s | 0.10 s | GC heap growth |
| `count(xs.filter(…))`, 2 million | 0.83 s | 0.10 s | the typed list became 2 million nodes just to count |
| map of 2 million keys, write and read | call stack exhausted | 2.7 s | cons-list map: quadratic, recursive copy |
| `xs#i = i / n` into float[10^7] | 29 s, 2.1 GB | 0.08 s, 0.1 GB | an exact quotient allocated per item (card exact-div) |

- **GC collector**: wasmtime's DRC collector (warp's only enabled one) ran this program about 3× slower than the
  copying collector, which also collects cycles. Cargo.toml enables `gc-copying` and `Collector::Auto` picks it.
- **Initial GC heap** (util.rs GC_HEAP_INITIAL_BYTES = 1 GB, reserved and committed lazily): the copying collector
  grows its heap too little, so a growing live set is copied again at every collection (400000 map entries: 6 s with
  64 MB, 0.3 s with 1 GB; the wasmtime CLI behaves the same with `-O gc-heap-initial-size`).
- **Two float[10^7] lists** (card gc-heap, 2026-10-09): `xs = float[n]; ys = float[n]` with dot, `sum(xs .* ys)` and
  `zs = xs .* ys` at n = 10^7 run (probes/memory/two_float_lists.warp). The "GC heap out of memory" seen on 2026-10-08
  came from the `float * list` error path, gone with 51e8d5721. The fill loop `xs#i = i / n` made an exact quotient per
  item, then its float (exact_div, exact_to_f64): 1.4 µs and garbage each, ~2 GB peak. Now (card exact-div) an Int
  quotient going straight into a float is `f64(a) / f64(b)` when both lie in [-2^53, 2^53) and b ≠ 0
  (big_int.rs emit_float_quotient): IEEE division rounds it once, as the exact quotient rounded. Otherwise, and for
  b = 0 (divide_by_zero), the exact way. The whole probe now runs in 1.4 s. Elsewhere `i / n` stays exact.
- **wasmtime's own build** (Cargo.toml `[profile.*.package.wasmtime] opt-level = 3`): release is size-optimized
  (`opt-level = "z"`, for the web build) and dev unoptimized; both ran the host side of GC-heavy programs slowly
  (2.5× and about 10× the CLI). The web build has no wasmtime, its size is unchanged.
- **Map variables are hash tables** (wasm_emitter/map_backend.rs): `m = {}` followed by `m[k] = v` with name keys is
  a `$NodeMap` (insertion-ordered keys and values, open-addressing slots, FNV-1a of the key's letters). It becomes
  the cons-list Node wherever the map is a value (a fresh copy, so value semantics hold); a lookup it cannot answer
  takes the generic way on that copy, with the same errors.
- **Texts grow in place** (text_concat): a left text ending where the heap starts gets right's bytes after it; a
  right text made just now (a character's bytes) already follows left, so nothing is copied. The left text's own
  bytes never change, so other values sharing them stay right.
- **Finger paint strokes** (card runtime-speed, 2026-10-10; probes/perf/finger_paint_bench.warp, release):
  2.6 ms → 0.4 ms a stroke. The guest profile (probes/perf/finger_paint_profile.warp, wasmtime `--profile=guest`,
  summed by probes/perf/profile_self_time.py) blamed boxing, not pixels: `floor(v) as int` built an Int node
  (new_int), read it back and called exact_trunc; `radius^2` called exact_pow. Now `floor`/`ceil`/`round` of a float
  are F64Floor/F64Ceil/F64Nearest + a truncating cast to the i64, an `as int` of a builtin rounding call skips
  exact_trunc (a user or FFI function of that name keeps it), `x^2` is one multiplication (floats: F64Mul; ints:
  the overflow-checked `*`), and a float `as int` truncates natively. Then `a // b` on fixnums is inline
  (`(a - a % b) / b` beside the inline `%`), an Int assignment whose value is dropped (`i = start` of a range loop)
  stores the i64 without new_int, and a computed range end (`for col in cell(x-r)..cell(x+r)+1`) is evaluated
  once into `col·end` instead of every round (lowering/for_loop.rs), as a walked list already was.

## Measured as fine
Closure calls cost what definition calls cost: 50 million calls of `add=(a,b)=>a+b`, of a capturing `x=>x*k` and of a
`def` take 0.3 s each (6 ns per call), through `apply(f, x)` 0.42 s. No boxing showed on these paths, so the per-site
closure typing has nothing to win here yet. Loops over typed lists, map, append: 2 million items in under 0.1 s.

## Still slow
A map that is not a hash table variable (a literal start `{a:1}`, a parameter, a global, one updated by `+=` or with
dynamic keys) is still the immutable cons list: n keys cost n². Each use of a hash-table map as a whole value
converts it (O(n)); a loop that prints or passes the map each round pays that every round.
Finger paint after runtime-speed: `dot(col, row)` passes Ints to `cell(v)`, whose v is a float: int → f64 → floor →
int per pixel (a per-argument-kind specialization of functions would drop it); a function whose loop is its last
value (`circle`) boxes that value per row although every caller drops it; int list get/set go through the Node layer.

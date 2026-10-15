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
| 100 KB literal on one line, 2200 records (compile, debug) | 62 s, 1.3 GB | 8.6 s, 0.13 GB | each word read the rest of its line, each LineInfo kept the whole line, signal_values copied the tree per level (card big-literals) |

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
- **Map nodes get an index** (card runtime-speed, 2026-10-12; wasm_emitter/map_index.rs): a map held as a cons list (a
  parameter, a returned map, an object) that map_find or field_with walks a second time, with 16 or more entries, gets
  a `$NodeMap` from name to entry in globals (two slots). field_with sets the entry or fields_grow appends after the
  remembered last cell; any other in-place change of links or items forgets the indexes (forget_list_cursor,
  list_with_at). 10000 inserts into a parameter: 4 s → 8 ms; 10000 lookups in a parameter: 1 s → 4 ms; in a returned
  map: 0.86 s → 5 ms (scratch probes like tests/lists/test_map_index.rs).
- **Characters of a text** (same card): `for ch in text` asked text_grapheme_count every round, which walks the text.
  The last counted (start, end) keeps its count in globals (texts never change in place), so 8000 characters:
  1.2 s → 0 ms (tests/text/test_for_each_char.rs).
- **Sorting in n log n** (same card): list_sort was a recursive insertion sort (call stack exhausted at 8000 items),
  `sort_by` a lowered insertion sort. Now both use list_sorted_by (library_ops.rs), a stable bottom-up merge sort over
  node arrays. A key function of one item (`xs.sort_by(p => p.k)`) is lowered to its keys computed once, then
  list_sorted_by(list, keys) (lambdas.rs key_sort_template). 30000 texts: 1 ms. A comparator of two items
  (`by: (a b) => a > b`) still takes the lowered insertion sort.
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
  once into `col·end` instead of every round (lowering/control/for_loop.rs), as a walked list already was.
- **Finger paint frames** (same card): `show()` passed draw's typed `canvas_pixels` to paint as a value, so each frame
  built a million Int Nodes (int_list_as_node) only for list_to_ints to read them back. Now a typed int list crosses
  to the host words in host.rs INT_LIST_WORDS (paint, sound) as its `$IntList`, and list_to_ints copies its array.
  Release bench: show 96–120 ms → 40 ms (headless; what's left is mostly the PNG write).
- **Drawing on the GPU** (card gpu-draw, 2026-10-12; notes/drawing.md "On the GPU"): in a window, `use draw` sends
  shapes (24 bytes each) instead of the canvas's pixels. Finger paint, quick-release, 1024×768 window: show 29–45 →
  0.2 ms, clear 10–29 → 0–1 ms, canvas() 25–46 → 0 ms (no pixel list allocated).

## Measured as fine
Closure calls cost what definition calls cost: 50 million calls of `add=(a,b)=>a+b`, of a capturing `x=>x*k` and of a
`def` take 0.3 s each (6 ns per call), through `apply(f, x)` 0.42 s. No boxing showed on these paths, so the per-site
closure typing has nothing to win here yet. Loops over typed lists, map, append: 2 million items in under 0.1 s.

## Still slow
Each use of a hash-table map as a whole value converts it (O(n)); a loop that prints or passes the map each round
pays that every round.
Finger paint after runtime-speed: `dot(col, row)` passes Ints to `cell(v)`, whose v is a float: int → f64 → floor →
int per pixel (a per-argument-kind specialization of functions would drop it); a function whose loop is its last
value (`circle`) boxes that value per row although every caller drops it; int list get/set go through the Node layer.
A shown frame in a window still goes through frame_bytes → a 4 MB pipe write → a texture upload in the paint-window
child process (not measured; WebGPU directly would drop it).
Exact decimals (card decimals-exact, 2026-10-10, release): `x = 0.5` is an exact ratio again. exact_add/sub/mul
take operands that are inline ratios or Ints within ±2^31 in i64 with an i64 gcd (exact.rs small_shortcut); the
generic int_gcd/ratio_new path (36 %/41 % of a stroke's profile) stays for bigger ones. 1M adds of 0.5
(probes/perf/decimal_hot_loop.warp): float 4 ms, exact 135 → 11 ms. A finger paint stroke: float 0.3 ms, exact
13 → 5 ms; what's left is circle's per-pixel `col + 0.5 - x` in exact math (a scaled-decimal i64 representation, or
hoisting the conversion into the float path, would drop it).
Everyday operations (card runtime-speed, 2026-10-12, quick-release; probes/perf/runtime_mix.warp, n=200k, ms):
push 1, index read/write 1, typed write 2, map 2, filter 3, concat 30 → 0 (int_list_concat: array.copy into an array
variable), slice 5, map insert 12 (20k), map lookup 7, sort 11. Left: slice, map insert/lookup; sort is now n log n (above), except with a two-item comparator.
Compile time (samples/visualizer.warp, CPU s): 0.84 → 0.50. The passes that need only names/params/bodies take
analyzer::function_definitions; a full analyse_user_functions runs per changed program state (the memo misses on
each), so a pass asking for one paid ~5–60 ms. Profile with probes/perf/xctrace_summary.py (unstripped build:
CARGO_PROFILE_QUICK_RELEASE_STRIP=false; --callers <name> shows who pays). Left: module loading of std (~190 ms of
samples), emit ~120 ms, Node clone/drop ~250 ms, the remaining full analyses (lambdas, closures, list_phrases).
Then 0.51 → 0.43 (2026-10-12): word_slices asked soft_keywords::program_names (a walk of the whole program) at
every node, and other passes once per word of their word lists (std module names, number words, emit verbs, sound
words with called_names each); they now take names_given / called_names once. modules::defined borrows the
statements (statements_of) instead of cloning the program. Then 0.38: closures::tail asks arrow_lambda of every
block, which copied the block (block_lambda) and searched it for `$0`…`$9` to rule out a Swift closure; a block
without a `$` name now returns at once (shorthand_block_lambda). Then 0.34: user_function takes the lifted body
instead of copying it again, and the return kind reads only the last statement (event_signals::last_statement)
instead of copying all of them. std_module_definitions parsed all 24 std modules on first use (~35 ms); each
module is now parsed when first asked: a tiny program compiles in 0.02 s instead of 0.04.
Tried, no gain: a per-thread cache of parsed std modules (insert_module_classes and the loader parse the same
source): cloning the tree costs about what parsing it does (visualizer 0.30 → 0.29 s), not kept.
Start of samples/gpu_visualizer_warp.warp (2026-10-15, card shader-start): the time to the first frame is the
compile, not the GPU. The first paint makes the shader in ~0.2–0.5 s and later paints take ~0.1 s, both for the WGSL
sample too (probes/perf/shader_start/*_first_frame.warp print the ms to the first paint, the first and the second
paint). Compiling takes 1.23 s of CPU in release against 0.63 s for samples/gpu_visualizer.warp; in the debug build
5.4 s against 3.8 s (the ~5 s the user saw through a debug try line). 43 % of the samples are in
analyse_user_functions (function_body_scope → collect_variables, refine_return_kinds → settled_return_kind): 13
vector functions calling each other.
Program cache (card cache-compiled-shader, src/run/program_cache.rs): `warp <file>` keeps the compiled module next to
the machine-code cache, so a second run skips the compile: gpu_visualizer_warp with `3`, debug build, 10.2 s → 5.9 s
wall (9.5 → 5.2 s CPU; the rest is the 5 s of music). A compile that prints anything (a hint!) is not kept, so the
sample was made hint-free (cosine/sine/exponential, if-then-else). Probes: probes/program_cache/cli_cache.sh (hits,
misses on changed program / used file, subfolder `use` not kept), probes/program_cache/samples_twice.sh.

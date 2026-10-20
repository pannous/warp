# Performance: what the benchmarks found

Benchmarks: `probes/night/bench_lists.sh [n]` (lists, closures, maps, text building), run with the release CLI
(`WARP=…/release/warp`); probes/perf/ (runtime_mix.warp, finger_paint_bench.warp, decimal_hot_loop.warp …). Keep n
unknown at compile time when timing one idiom by hand (`n=2000000+random_below(1)`), otherwise constant evaluation
may hide the loop. Profile the generated module apart from warp: `warp compile x.warp`, then
`wasmtime run -W gc=y,function-references=y,exceptions=y,tail-call=y --profile=guest,out.json --invoke main x.wasm`
(Firefox profiler; probes/perf/profile_self_time.py sums it); `/usr/bin/sample <pid>` on the debug CLI shows the
host's side; compile time with probes/perf/xctrace_summary.py (unstripped build: CARGO_PROFILE_QUICK_RELEASE_STRIP=false;
`--callers <name>` shows who pays).

## What keeps the runtime fast (each fixed a measured hot spot)
- **GC collector**: Cargo.toml enables `gc-copying` and `Collector::Auto` picks it (wasmtime's DRC collector ran ~3×
  slower and collects no cycles). **Initial GC heap** util.rs GC_HEAP_INITIAL_BYTES = 1 GB, reserved and committed
  lazily: the copying collector grows its heap too little, so a growing live set is copied again at every collection.
- **wasmtime's own build** `[profile.*.package.wasmtime] opt-level = 3`: dev is unoptimized and web-release
  size-optimized; they ran the host side of GC-heavy programs 2.5–10× slower.
- **The native release is opt-level 3** (card release-profile-slower, 2026-10-17): at opt-level z the compiler itself
  lowered and emitted 3.3× slower (a 300-function program 5.0 s → 1.5 s user); the binary grew 15.5 → 28.9 MB. Only
  the playground's warp.wasm, downloaded by every visitor, keeps z (profile web-release).
- **Map variables are hash tables** (wasm_emitter/map_backend.rs): `m = {}` followed by `m[k] = v` with name keys is
  a `$NodeMap` (insertion-ordered keys and values, open-addressing slots, FNV-1a of the key's letters). It becomes
  the cons-list Node wherever the map is a value (a fresh copy, so value semantics hold); a lookup it cannot answer
  takes the generic way on that copy, with the same errors.
- **Map nodes get an index** (wasm_emitter/map_index.rs): a map held as a cons list (a parameter, a returned map, an
  object) that map_find or field_with walks a second time, with 16 or more entries, gets a `$NodeMap` from name to
  entry in globals (two slots). field_with sets the entry or fields_grow appends after the remembered last cell; any
  other in-place change of links or items forgets the indexes (forget_list_cursor, list_with_at).
- **Characters of a text**: the last counted (start, end) keeps its grapheme count in globals (texts never change in
  place), so `for ch in text` does not walk the text every round.
- **Sorting in n log n**: list_sort and `sort_by` use list_sorted_by (library_ops.rs), a stable bottom-up merge sort
  over node arrays; a key function of one item is computed once per item (lambdas.rs key_sort_template). A comparator
  of two items (`by: (a b) => a > b`) still takes the lowered insertion sort.
- **Texts grow in place** (text_concat): a left text ending where the heap starts gets right's bytes after it; a right
  text made just now already follows left, so nothing is copied. The left text's own bytes never change.
- **Counting a filter** of a typed list does not build Nodes; `t += "x"` and `"\(i)"` in loops are linear.
- **Int quotient into a float**: `xs#i = i / n` into float[n] is `f64(a) / f64(b)` when both lie in [-2^53, 2^53) and
  b ≠ 0 (big_int.rs emit_float_quotient; IEEE division rounds once, as the exact quotient rounded); elsewhere `i / n`
  stays exact.
- **Rounding and powers**: `floor`/`ceil`/`round` of a float are F64Floor/F64Ceil/F64Nearest + a truncating cast; an
  `as int` of a builtin rounding call skips exact_trunc; `x^2` is one multiplication; `a // b` on fixnums is inline;
  an Int assignment whose value is dropped stores the i64 without new_int; a computed range end is evaluated once
  into `col·end` (lowering/control/for_loop.rs).
- **Typed int lists to host words** (host.rs INT_LIST_WORDS: paint, sound) cross as their `$IntList`, no Int Nodes.
- **Drawing on the GPU** (notes/drawing.md "On the GPU"): in a window `use draw` sends shapes (24 bytes each), not
  pixels.
- **fast Ints**: a whole Int expression in a fast region is machine code (big_int.rs emit_machine_root): when its
  leaves are proven fixnums or Int locals, one test of those locals and the result picks plain i64 code. `byte_at`
  and `memory_byte` are proven 0…255. `x: i64` (`x: fast int`) wraps.
- **Byte loops**: `byte_at(t, i)` and `t.bytes` of a text are inline (text_builtins.rs emit_byte_at_inline,
  list_dispatch.rs emit_byte_count_inline).
- **Exact decimals**: exact_add/sub/mul take operands that are inline ratios or Ints within ±2^31 in i64 with an i64
  gcd (exact.rs small_shortcut); the generic int_gcd/ratio_new path stays for bigger ones. 1M adds of 0.5: float 4 ms,
  exact 11 ms.
- **Big literals**: a 100 KB literal of 2200 records compiles in seconds (each word reads only its token, LineInfo
  keeps no whole line).

- **A list's items in one walk** (list_runtime.rs emit_item_cells_walk): list_items fills an array that grows when
  full (grown_when_full, push's growth too) instead of walking the cells twice, and node_count counts with
  list_item_count, no array: pure-only passes.warp 48 → 41 s.
- **Spelled cases**: `op == Op.Add` compares with "+" (a written case folds to its spelling), and Op·text gives a
  text back before its ~70 match arms (plain_enums.rs spelled_text, node_cases.rs spelling_functions).

Closure calls cost what definition calls cost (6 ns per call, mostly fuel); loops over typed lists, map, append:
2 million items in under 0.1 s.

## Still slow
- Fuel (card fuel-metering, probes/fast_int/fuel_cost.sh): every run meters fuel (warp-runtime engine::fueled_config),
  and that is most of what a call or a branch costs: 40M calls of a tiny function take 49 ms in the wasmtime CLI,
  218 ms with `-W fuel`, 208 ms in warp. Epoch interruption would be the cheap stop, but it is not deterministic.
- Each use of a hash-table map as a whole value converts it (O(n)); a loop that prints or passes the map each round
  pays that every round.
- Everyday operations (probes/perf/runtime_mix.warp, n=200k, ms): push 1, index read/write 1, typed write 2, map 2,
  filter 3, concat 0, slice 5, map insert 12 (20k), map lookup 7, sort 11. Left: slice, map insert/lookup.
- Finger paint: `dot(col, row)` passes Ints to `cell(v)`, whose v is a float (int → f64 → floor → int per pixel; a
  per-argument-kind specialization would drop it); a function whose loop is its last value boxes that value per row
  although every caller drops it; int list get/set go through the Node layer; circle's per-pixel `col + 0.5 - x` is
  exact math (a scaled-decimal i64 representation would drop it). A shown frame in a window goes through frame_bytes →
  a 4 MB pipe write → a texture upload in the paint-window process.

- The self-hosted stages (pure-only passes.warp, ~41 s warm): parse stage profile (data/speed/prof): closure entries
  ~19% self (calls through a function value: bottom_up visits every node in every pass), text_literal 14%,
  list_items + list_item_count 14% (scope.warp nodes_kept keeps its stack in a cons list: `unvisited#(#unvisited)` and
  a slice per step), text_bytes_equal + values_equal ~9%.
- Tried, no gain: Binaryen wasm-opt -O2/-O3/-Oz (Cranelift does that work); only -O3 with aggressive inlining runs
  12-19% faster, at 2.4x size and +1.2 s wasmtime compile (card wasm-opt-inline, probes/perf/wasm_opt_bench.sh).
  wasmtime's own inlining measured slower.

## Timing the self-hosted stages
- Time against a frozen copy of lib/ (`cp -R lib scratch/speed/lib_frozen`, `WARP_LIB=` it, run in its compiler/):
  every push to lib/compiler by another session recompiles the four stage programs (~40 s) in the middle of an A/B,
  which once made a 15% swing look like a result. Discard the first run of each binary (it compiles the stages).
- The Mac is shared: alternate the two binaries round by round and compare `user` time, not `real`.
- `WARP_PHASE_TIMES=1` names each emission rerun with the needs that caused it (6 per cold stage compile, ~6.7 s).

## Compile time
- Shared analysis state: function kinds and closure targets are `Rc` (analyzer FunctionKinds, ClosureTargets); passes
  needing only names/params/bodies take analyzer::function_definitions; name sets (names_given, called_names) are
  computed once per pass, not per node; std modules are parsed when first asked (a tiny program compiles in 0.02 s).
- Left, each ~20–50 ms on samples: register_user_function copies each body (and every pass asking
  function_definitions registers again), each implicit module's test walks the program for its called names (router,
  units, agent), naming_changed walks every body per round, loading std modules (~220 ms with parsing them),
  analyse_user_functions on programs of many mutually calling functions (collect_variables, refine_return_kinds).
- Tried, no gain: a per-thread cache of parsed std modules (cloning the tree costs about what parsing it does).
- Program cache (src/run/program_cache.rs): `warp <file>` keeps the compiled module next to the machine-code cache, so
  a second run skips the compile; its hints, warnings and assumptions are said again by each run. The working
  directory is part of the key (probes/pure_chain/run.sh runs emit.warp from its own folder for that). Probes:
  probes/program_cache/cli_cache.sh, probes/program_cache/samples_twice.sh (samples whose two runs differ do so
  without the cache too: timings, random weights, stored rows, GPU frame counts).

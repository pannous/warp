# Compile speed of the warp compiler (card rebuild-pure-slowdown, 2026-10-30/31)

A rebuild grew from ~140 s (10-29) to 380–420 s (10-30). Measured on compiler.warp compiled by the compiler itself,
CPU seconds between phase markers:

| state | compile CPU |
|---|---|
| 10-30 evening | 607 s |
| method_calls module rule fixed (b15a98773e) | 285 s |
| with_expected_type_uses fixed (9b01b1bb2a) | 230 s |

## How to measure
- Profiling, first: `scripts/profile.sh [-s <seconds>] <warp words…>`, e.g. `scripts/profile.sh -s 90 rebuild pure`
  (WARP_LIB/WARP_SEED as for any run). The run gets WARP_PROFILE=perfmap (the native driver writes
  /tmp/perf-<pid>.map), macOS's sample takes its main thread's stacks, and the script prints each wasm function's own
  samples and those with its callees: no rebuild, no markers. `-r <pid>` reports a kept data/profile/<pid>.sample
  again. sample manages ~30 ms a sample on wasmtime's threads, so profile a minute or more. Its first find: 55% of a
  parse in alias_at → longest_spelled_at, 39% in operator_at → spelled_operator_at (2026-10-31).
  Cost (2026-10-31, bench.warp on parser.warp, ~94 s CPU): WARP_PROFILE=perfmap alone changes nothing (93.3 vs 93.5 s
  CPU); `sample` attached for the whole run keeps CPU the same (94.0 s) and adds ~10–15% wall (108 vs 94–95 s) while
  it pauses the thread. On that run it took 76 k main-thread samples in 108 s, ~1.4 ms each.
- Phase markers (card phase-markers): `timed(label)` (core/passes/passes.warp) prints `§T <milliseconds> <label>`
  when WARP_TIMES is set, silent otherwise, where a phase starts: compiler.warp's compile (implicit uses,
  checked_program, emit, emit done), with_used's steps, with_uses' (used files, module texts, each file's parse), each
  baked table's computing, each pure pass by name. A build run with WARP_TIMES=1 keeps them in its record's build.log
  (notes/compiler_rebuilds.md "Build records"; stage two's record for compiler.warp compiled by the tree's own
  compiler); `scripts/phase-times.py <build.log>…` sums the wall spans per label, biggest first, up to `emit done`.
  CPU and hotspots are the record's time and profile.txt: never build again to measure.
- A synthetic program at 100/200/400/1600 statements shows superlinear terms quickly with the installed warp
  (`/usr/bin/time -p`, user CPU): `def gN(xs: list): int := xs.f(N) + 1` against `f(N) + 1`.
- Rules in lib/sugar/ are not baked: switching one off in a worktree's lib (WARP_LIB) needs no rebuild, so a rule's
  cost is bisected in seconds.
- The wasmtime CLI profiler trapped on the compiler (card wasmtime-cli-refusal); the markers need no profiler.

## What it was
1. lib/sugar/method_calls.warp `$m.$f($a...) … $module = module_named($m, $program)` (e3ab37c931): `$m is not
   value` passes for a parameter (rule_scope is the program's), so module_named ran at nearly every `xs.f()`, and each
   call walked the whole program (names_bound_in) and read a module file. Quadratic: late_sugar 300 → 5.6 s.
   Fix: names_bound_in_kept (one-entry same_node cache) and used_module_text reads each module once.
2. with_expected_type_uses (`sleep(3ms)` finding units.warp) parsed a called word's definition text for every argument
   of every call (2956 calls): 54 → 0.2 s. Fix: heads parsed once per word, only for arguments naming an unknown word.
3. Not it: indexing builtin lines by name (lines_of filtered ~1000 lines per lookup) saved only 3 s; not landed.

## The lesson
A rule word or pass helper that reads the whole program (`$program`, names_bound_in, file reads, parses) must compute
once per program, never per node or call site. Cheap `where` guards don't save a rule whose guard passes broadly.

4. called_only (warp-quality, 94a0dfeea8): each called word scanned all statements, the pending list was copied per
   step: 24 → 1.1 s.
5. The parser's per-token cost (card compiler-parse-timing): every token tried every operator alias in turn
   (alias_at → longest_spelled_at, 55% of a parse), and each operand found superscripts by walking the superscript
   table text (position_in, ~25% of what was left). Fix: the aliases, truth glyphs, negated spellings and test words
   indexed by first character once (parser.warp by_first_character, starting_here), the superscript tables as maps
   (character_index). The 117 core files under 60 KB parse to the same trees, 30.4 → 16.0 s CPU; compiler.warp
   compiled by warp.wasm 616 → 556 s CPU, back to back on a loaded machine.
   Found with warp-quality's WARP_PROFILE=perfmap (scripts/profile.sh) on `warp parse-code "<2000 lines>" out.bin`;
   a parser timing needs no `warp parse` (its print is quadratic) and no compiler build per probe.

## Left (main 42a19df29a, stage two 2efb9b98f2f3eca9: 166 s CPU, 580 MB; was 820 s this morning)
No single fix of 5 s or more is left without a design change (cards):
- emit 46 s (27%): wasmtime's is_subtype libcall and GC (warp-quality, card emit-wasmtime).
- nodes_in lists (card builds-list): keep_nodes 13% of the compile across ~60 callers, none over 8%; half GC.
- rule tries (card sugar): rewritten_program 8.7 s, sugar 9.7 s, each kept rule tried at each node.
- LIBRARY_PARSES 7 s per compiler build (card library-parses-cost): the bake parses every std module anew.
- then lambdas 7.5 s, arity_overloads 6.2 s (mostly nodes_in), first/late/iteration/comprehension sugar 4–5 s each.
`warp parse` printing a big tree is quadratic (card parse-print-quadratic); the parse itself isn't.

## Three walks (2026-10-31, found on the def-first measurement)
Self-check's stage two (compiler.warp compiled by the tree's own warp.wasm), both on the GC growth fix (1cd7a5c2e3:
wasmtime grew a copying heap only when its active half was full; that run was 831 -> 227 s CPU and ~80% GC before),
records /Volumes/Build/compilers/{2cd7795a6e4464f8,044362cbf97a7d00}/stage-two.record: CPU 214.2 -> 194.9 s (-9.0%),
peak RSS 942 -> 625 MB (-34%), all from two walks:
- lambdas' captures_assigned_later gathered, for every statement, the assignments of all later ones (quadratic, 10% of
  the compile): each name's last assignment is gathered once (lambdas 18.8 -> 7.2 s).
- each sugar step listed the program's nodes five times for its rule lookups, now once (sugar steps, type_tests -8 s).
- index_positions copied every name the program assigns (most counting nothing) into each function's scope: a scope
  now passes on only the names that count (a function's own `i = t.find(x)` counts, its local): index_positions
  17.9 -> 1.4 s (stage two of 9a845e21fccdf599, on a loaded machine).
- type_tests and plain_enums copied the whole program's scope into each function: a function gets only the
  entries its own names ask for (scope.warp names_seen): type_tests 10.0 -> 0.9 s, plain_enums 10.7 -> 3.5 s; stage two
  166 s CPU, 580 MB (2efb9b98f2f3eca9).
def-first (every definition starting with `def`) gained nothing measurable and is parked on branch
function-definition-core; its two collection passes were not built (the collectors they'd replace are <0.5% each, the
definition gatherers ~1-1.7%; a table collected after with_used goes stale as later passes add, rename and lift
definitions; user: discuss later). Left: emit 44 s (22%), index_positions 18 s, plain_enums 9 s, LIBRARY_PARSES 7 s.

### Emit (~42 s; warp-quality 2026-10-31, profile.sh -f typed_wasm_module on rebuild pure, 39053 samples)
- 26% of emit is wasmtime's is_subtype libcall (StoreOpaque::is_subtype_cached): a `ref.test`/`ref.cast` whose
  types are not the same takes it, a failing test always. The runtime's number and node type tests do so on every
  call: node_type_test 24% with callees, ratio_node 27%, number_byte 20% (a value tested for a ratio, a box…).
  Fixed (card emit-wasmtime): wasmtime tests a final type by index equality, an open one of another type through the
  libcall; every class struct was open, so instance_place / instance_field_at tested each class in turn and
  is_class_instance the open $Instance (55% of the libcall). A class nobody extends now has a final struct and
  is_class_instance tests each class's: stage 3 (the compiler compiling itself through code its own emitter wrote)
  is_subtype 8.5% → 1.4%, 254.6 → 242.2 s CPU. The rest is the cast to the open $Node of a $MetaNode or an instance
  (node_equal, node_text). Measure an emitter change at stage 3: stage 2 runs the seed's code, and the store's
  warp-self.wasm is a copy of warp.wasm (data/emit-wasmtime/measure.sh).
- 19% is the copying collector (gc_alloc_raw → do_gc → collect_increment, CopyingHeap::forward/scan_field): a bigger
  initial GC heap (warp_config_gc_heap_initial_size_set) for the compiler may cut it, card wasmtime-copying-collector.
- The emitter's own top paths: compile_function → statement → effects → joint_kind → instance_of_fields →
  forwarded_kinds → no_field_unshared → call_values (60%), field·class_defaults 44%, emitter_kind 46%; one widening
  round only. By markers: 2975 functions, median 4.4 ms; a one-line function ~1.5 ms, a lambda ~0.3 ms.

## Marker pitfalls
- A marker must stand before the work it names: `f(label, computed)` prints after `computed` ran (timed_bake did).
- phase-times.py stops at a log's first `emit done`.
- The marks are printed (stdout): WARP_TIMES set for a test or gate run makes every case's output differ; set it only
  for the build or self-check measured.

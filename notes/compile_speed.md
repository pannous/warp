# Compile speed of the warp compiler

## How to measure
- Profiling, first: `scripts/profile.sh [-s <seconds>] <warp words…>`, e.g. `scripts/profile.sh -s 90 rebuild pure`
  (WARP_LIB/WARP_SEED as for any run). The run gets WARP_PROFILE=perfmap (the native driver writes
  /tmp/perf-<pid>.map), macOS's sample takes its main thread's stacks, and the script prints each wasm function's own
  samples and those with its callees: no rebuild, no markers. `-r <pid>` reports a kept data/profile/<pid>.sample
  again. sample manages ~30 ms a sample on wasmtime's threads, so profile a minute or more.
  Cost (bench.warp on parser.warp, ~94 s CPU): WARP_PROFILE=perfmap alone changes nothing (93.3 vs 93.5 s
  CPU); `sample` attached for the whole run keeps CPU the same (94.0 s) and adds ~10–15% wall (108 vs 94–95 s) while
  it pauses the thread. On that run it took 76 k main-thread samples in 108 s, ~1.4 ms each.
- Phase markers (card phase-markers): `timed(label)` (core/passes/passes.warp) prints `§T <milliseconds> <label>`
  when WARP_TIMES is set, silent otherwise, where a phase starts: compiler.warp's compile (implicit uses,
  checked_program, emit, emit done), with_used's steps, with_uses' (used files, module texts, each file's parse), each
  baked table's computing, each pure pass by name. A build run with WARP_TIMES=1 keeps them in its record's build.log
  (notes/compiler_rebuilds.md "Build records"; stage two's record for compiler.warp compiled by the tree's own
  compiler); `scripts/phase-times.py <build.log>…` sums the wall spans per label, biggest first, up to `emit done`.
  CPU and hotspots are the record's time and profile.txt: never build again to measure.
- WARP_TIMES also prints `§R <path>` for each file a compile reads (library.warp file_source): the audit tool of
  no-lib-reads.
- A synthetic program at 100/200/400/1600 statements shows superlinear terms quickly with the installed warp
  (`/usr/bin/time -p`, user CPU): `def gN(xs: list): int := xs.f(N) + 1` against `f(N) + 1`.
  test/bench/compile_scaling.sh measures it (instructions retired, warm): hello beside 400 more indexed words, and
  generated programs of 100/1000/2000 lines (four statement shapes, each using the ones before).
- Rules in extensions/sugar/ are not baked: switching one off in a worktree's lib (WARP_LIB) needs no rebuild, so a rule's
  cost is bisected in seconds.
- The wasmtime CLI profiler trapped on the compiler (card wasmtime-cli-refusal); the markers need no profiler.
- Profiling hello: `sample` attaches ~0.2 s late, so profile.sh catches only a hello's tail; use WARP_TIMES phases
  for it, or a longer program.
- Exact call counts per function aren't measurable today: wasmtime counts nothing, and binaryen's --log-execution
  needs an import the driver doesn't answer. Summed samples over many runs stand in.

## The lesson
A rule word or pass helper that reads the whole program (`$program`, names_bound_in, file reads, parses) must compute
once per program, never per node or call site. Cheap `where` guards don't save a rule whose guard passes broadly.
- Pitfall: `x in list` compares node by node (node_equal, NFC for texts): a list of names searched per symbol or per
  statement is the usual quadratic. A map (`word_set`) of the names, made once outside the loop.
- A hook on a hot path keyed by what starts its form costs one lookup, a list of hooks costs each one's own scan.
- wasmtime runs a `ref.cast` even when the value is statically of the target type (200 M casts of a
  $MetaNode to $Node: 1.13 s, none 0.19 s). Casts stay
  only where an anyref enters (emitter `entered_node`): a list item (`item_read`), a $Node's data, a host word's node
  result, a call_ref result. Host imports keep anyref; closure entries and class words return anyref (a class word's
  place/set is an i31). A closure's item 0 is a $Closure: read by `closure_slot`, never through the node cast.
- The kept map (lexer.warp code_only_known) is a global holding every source of a compile: bounded, as each
  compile gets a fresh instance (the native driver a fresh store per run, the playground a fresh instance per compile,
  web/playground/driver.js). A host keeping one compiler instance across compiles would grow it: clear it there.
- Pitfall: a text holding wasm bytes is asked for its byte_size, never compared (`== ""`): text equality
  normalizes (NFC) and trapped on the bytes.
- WARP_HINTS=0 / --no-hints only filters hints when printing (core/compiler.warp said_at); the checks still run, and
  skipping their producers would save under 0.5 %: not built.

### Emit
- 26% of emit is wasmtime's is_subtype libcall (StoreOpaque::is_subtype_cached): a `ref.test`/`ref.cast` whose
  types are not the same takes it, a failing test always. The runtime's number and node type tests do so on every
  call: node_type_test 24% with callees, ratio_node 27%, number_byte 20% (a value tested for a ratio, a box…).
  Fixed (card emit-wasmtime): wasmtime tests a final type by index equality, an open one of another type through the
  libcall; every class struct was open, so instance_place / instance_field_at tested each class in turn and
  is_class_instance the open $Instance (55% of the libcall). A class nobody extends now has a final struct and
  is_class_instance tests each class's: stage 3 (the compiler compiling itself through code its own emitter wrote)
  is_subtype 8.5% → 1.4%, 254.6 → 242.2 s CPU. The rest is the cast to the open $Node of a $MetaNode or an instance
  (node_equal, node_text). Measure an emitter change at stage 3: stage 2 runs the seed's code, and the store's
  warp-self.wasm is a copy of warp.wasm.
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
- WARP_TIMES marks are printed where a phase *starts*, so the time after `pass sugar` is sugar's, not the next pass's.

## The compiler's machine code (card wasm-becomes, measured on warp.wasm 2.5 MB, wasmtime 50)
- warp.wasm compiles to 38.7 MB of Cranelift output: .text 26.3 MB, then .wasmtime.stackmap 4.5, .addrmap 4.0, .traps
  1.2, .eh_frame 1.0. The GC code itself is the size (~10 bytes of machine code per wasm byte); no config shrinks it.
- Config levers, each alone (data/wasm-becomes/measure.c compiles a module under one config): native_unwind_info off
  37.7 MB (−1 MB, the .eh_frame); cranelift opt level speed_and_size 38.7 MB (no change, compile +40%); opt none
  48.6 MB; regalloc single_pass 57.9 MB; all fast settings 77.9 MB (compile 5.2 s instead of 10). generate_address_map
  is not in the C API (−4 MB of file at most). Parallel compilation only changes the first compile's wall time.
- Resident: a running warp holds the whole 25 MB .text resident, but clean and file-backed (vmmap: resident 25.0M,
  dirty 0): shared by concurrent warp processes and reclaimable, so not the private memory it looks like. Of the 11.8 MB
  of side tables ~3.5 MB is touched. Hello: 0.15 s wall, 108 MB max RSS, mostly that mapping and the dylib.
- So the levers are not worth it: the only size cut (unwind info off, −2.6% of file, ~0 resident) costs native
  backtraces. A smaller compiler comes from less wasm (fewer or smaller functions), not from wasmtime's config.

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
- Phase markers: a compiler built with `timed(label)` calls (`print "§T " + label` when WARP_TIMES is set) in
  passes.warp lowered_node (per pass index), with_used, compiler.warp; branch compile-times (scratch, never landed).
- `data/phase_times.py <command…>` (in that branch) reads the process's CPU (ps) at each marker and prints the spans,
  biggest first; it stops after `emit done`. Run: build the marked compiler, then
  `WARP_TIMES=1 WARP_SEED=<marked warp.wasm> python3 data/phase_times.py warp rebuild pure`.
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

## Left (main 8a45e5…: lowering 176 s up to checked_program, emit ~40 s)
The parser is the biggest single cost, ~62 s:
- the used core files: 30.9 s, linear (parser.warp 4819 lines 5.6 s, ~19 µs per byte; card compiler-parse-timing)
- SLICE_PARSES 18.6 s: every library definition parsed again, one by one, after LIBRARY_PARSES (12.4 s) parsed each
  whole file (card slice-parses-definitions)
Then index_positions 17 s, lambdas (pass 23) 11 s, sugar (pass 22) 8 s; emit (card emit-cpu-profile, warp-quality).
with_uses did not regress on main: 65 s, as before; a measurement beside other builds inflated it to 75–93 s.
`warp parse` printing a big tree is quadratic (×4 the file: 353 s, card parse-print-quadratic); the parse itself isn't.

## Marker pitfalls
- A marker must stand before the work it names: `f(label, computed)` prints after `computed` ran (timed_bake did).
- phase_times stops only at `emit done`; without that marker the rebuild's tail lands in the last label.

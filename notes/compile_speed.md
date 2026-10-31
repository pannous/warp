# Compile speed of the warp compiler (card rebuild-pure-slowdown, 2026-10-30/31)

A rebuild grew from ~140 s (10-29) to 380–420 s (10-30). Measured on compiler.warp compiled by the compiler itself,
CPU seconds between phase markers:

| state | compile CPU |
|---|---|
| 10-30 evening | 607 s |
| method_calls module rule fixed (b15a98773e) | 285 s |
| with_expected_type_uses fixed (9b01b1bb2a) | 230 s |

## How to measure
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

## Left (230 s)
with_uses 44 s, emit 41 s, called_only 24 s, baked LIBRARY_SLICES 17 s, index_positions 14 s, lambdas (pass 23) 11 s.

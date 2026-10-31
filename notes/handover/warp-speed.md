# Handover: warp-speed (compile speed, rebuilds, seeds), 2026-10-30/31

Session warp-speed. Topic note with method and numbers: notes/compile_speed.md (read it first).

## Results landed on main
| commit | what | gain |
|---|---|---|
| e47a024c37 | rebake: baked library tables are data segments `warp rebake` rewrites (baked_segment, write_rebaked, baked-current) | rebuild 1541 → 718 s CPU (no second full compile) |
| 86d08b8474 | seed.lock + scripts/seed.sh: every commit names its seed; refresh-seed tags the source commit | seeds reconstructible per commit |
| b15a98773e | method_calls' module rule: names_bound_in_kept (same_node cache), used_module_text reads once | compiler.warp compile 607 → 285 s CPU |
| 35d7b5b7c3 | stored-compiler-rebake: a lib/-only change rebakes the stored compiler of the same code (store/code/<baked hash>) | lib-only rebuild ~164 s wall / 55 s CPU instead of a full build |
| 403af5d021 | rebake-incomplete: a stored compiler a pre-rebake seed built (5 of 7 segments) is compiled anew, not a kept failure | lib-only pushes never stuck |
| 9b01b1bb2a | with_expected_type_uses parses a called word's definitions once, only for arguments naming unknown words | 54 → 0.2 s; compile 287 → 230 s |
| dbb59216e1 | slice-parses-definitions: a library file parsed once in a compiler build, slices cut from the file's parse (classes matched by name: their node sits on the previous statement's line); definitions keep their file line (Supervisor decision) | ≤ 18.6 s per build (not yet measured on main) |
| 621048b5c8, 68d0d90d57 | notes/compile_speed.md | |
By others from my hand-outs: called_only 24 → 1.1 s (warp-quality, 94a0dfeea8); WARP_PROFILE=perfmap + scripts/profile.sh
(warp-quality, 910fb626e9; first find: 55 % of a parse in longest_spelled_at).

## Findings worth keeping
- The slowdown since 10-29 was library-side work done per call site inside the compile, not core features: rule words
  or helpers reading the whole program ($program, names_bound_in, file reads, parses) must compute once per program.
- Cheap `where` guards don't save a rule whose guard passes broadly (`$m is not value` passes for any parameter,
  since rule_scope is program-level). Card where-cheap-first / rule-condition-scope went to warp-quality.
- Parsing is linear in a compile (~19 µs/byte, parser.warp 4819 lines 5.6 s), but `warp parse` printing a tree is
  quadratic (×4 file = 353 s): never time the parser with `warp parse` (card parse-print-quadratic).
- Indexing builtin lines by name (lines_of) saved only 3 s: not landed. Text slicing is constant time, also non-ASCII.
- with_uses did NOT regress on main (~65 s both); measurements beside other builds inflate spans by up to 2×.
- A compiler built by a seed older than the rebake code carries 5 of 7 baked segments: check a compiler with
  `LC_ALL=C grep -ao 'warp\.baked [A-Z_]*' x.wasm | sort -u` (7 names = rebakeable). The seed's code decides, not its
  own segments.
- Seed quirks (cards): `"${#xs}"` and `"${12345}"` fail to interpolate, char * `t.length` fails, a bare `return`
  mixed with a value fails in primitive mode, locals named like words (length, count, next, last, line) collide.

## Where the 230 s go now (main before dbb59216e1; lowering ~176 s up to checked_program, emit ~40 s)
parsing the used core files 31 s (card compiler-parse-timing, warp-shaders) · SLICE_PARSES 18.6 s (fixed in
dbb59216e1) · LIBRARY_PARSES 12.4 s · index_positions 15–17 s · lambdas 11 s · sugar 8 s · emit ~42 s spread out
(main 22, uncalled functions 6.5, analyze 5.5, trap walks 3.8; ~1.2 ms fixed per named function; warp-quality's
emit-cpu-profile, data in data/emit-times*.log).

## In progress when handed over
1. Card phase-markers (just created, no branch yet): put the WARP_TIMES markers on main, silent when unset
   (Supervisor decision). Curated set from /Volumes/Worktrees/compile-times (scratch): compiler.warp (implicit uses,
   checked_program, emit, emit done), passes.warp `TIMES = env("WARP_TIMES") ?? ""`, `timed(label)` and
   `timed("pass " + "${pass}")` per pure pass (a function value prints its name), with_used's steps, uses.warp
   with_uses' steps, a mark BEFORE each baked table (`timed("computing X")`), checked_parse's per-file parse mark.
   Plus scripts/phase-times.py from compile-times data/phase_times.py (stops at `emit done`).
2. Card function-definition-core (user 2026-10-31 via Supervisor, decision def-first, user OK for the core change and
   both passes): (a) every function definition in core/ and lib/ starts with `def`: ~2542 top-level in core, ~1119
   top-level + 120 class methods in lib (`^\s*name(…)…:=`); lines led by print/once/type/raise/on/log/extend/test are
   real definitions too; dotted ones (`canvas.width :=`) aren't matched; @primitive lines start with @. Mechanical
   (fastmod), outputs unchanged. (b) right after folding, a pass collecting all classes and type definitions, then one
   collecting all functions, replacing the scattered collection points (Declared, class_methods, names_bound_in /
   statements walks) so later passes look up instead of rescanning. (c) measure compiler.warp's compile with
   WARP_TIMES and scripts/profile.sh before and after, per phase. Land in steps (def rewrite, then passes), each with
   smoke + self-check + gate tier 1.

## Cleanup left
- Worktrees: /Volumes/Worktrees/compile-times (scratch timing branch, never landed; its data/ holds phase_times*.out
  and the synthetic programs data/quad/, data/slicing/), /Volumes/Worktrees/slowdown-input, /Volumes/Worktrees/ocean-60fps
  (the 8337e2f64d check): remove after taking the markers over.
- The user still has to run `~/dev/angles/warp/scripts/seed-tags-to-sources.sh` (moves 25 old seed tags onto their
  source commits; the hook blocks the tag force-push for agents).
- Card rebuild-pure-slowdown is done; cards still open from this work: compiler-parse-timing (warp-shaders),
  parse-print-quadratic, emit-cpu-profile (warp-quality), wasmtime-cli-refusal, phase-markers, function-definition-core.

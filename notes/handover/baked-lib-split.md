# Handover: baked-lib-split (card baked-lib-split, decision core-takes-what-it-uses)

Goal (user 2026-11-09): nothing of lib/ is baked into the compiler. What the compiler needs of the library moves to
core/, the library builds on core's words (moves, never copies). Worktree /Volumes/Worktrees/baked-lib-split, branch
baked-lib-split.

## Done
- Step A, on main (1f5bbca15d): scripts/compiler-inputs.sh counts only true inputs (aliases.warp was never in
  warp.wasm's bytes; web.webidl was missing from the full hash). Checked by searching every lib file's lines in the
  installed warp.wasm: module_source texts (prelude, math, c, primitive, web.webidl, entities.tsv, modes.data) were
  baked whole; os/file/path/math/reals/case_table(s) were compiled in through `use`.
- Part 1, handed to the Integrator at ab1a7aab17 (smoke, self-check, gate tier 1 green):
  - data files live with their readers: core/passes/modes.data, extensions/entities.tsv,
    extensions/js_globals/web.webidl, extensions/primitive/prelude.warp
  - the compiler `use`s no lib module: core/host_io.warp (write, write_bytes, read_bytes, exists, list_files,
    home_expanded, home_folder, started_env), core/integers.warp (gcd), core/runtime/case_tables.warp; lib/host_io.warp,
    lib/integers.warp, lib/case_tables.warp are symlinks into core/ (a lib module's file include `use "compiler/x.warp"`
    does not bring core definitions in; a symlinked module does; uses are not transitive, implicit uses through the
    index cover plain calls, so the names are in rewrites.warp std_module_names). The emitter's bit_and/bit_or are
    the runtime instruction words bits_and/bits_or (core/runtime/instructions.warp, included by emitter.warp).
  - baked list 18 → 11 (lib: prelude.warp, math.warp, c.warp left).
- Part 2, P1+P2, commits d11d08d497 + 459585c1a2 (smoke, self-check, gate tier 1 green; handed to the Integrator):
  - core/analysis/builtins.warp and math_builtins.warp: the typed builtin lines (`from warp`, `from emitter`,
    `from 'm'`), the only typed lines the compiler reads (builtin_signatures.warp typed_texts_seen).
  - prelude, math and c are read from disk when a compile first asks (prelude_text(), math_text(),
    std_module_text("c")); their derived tables are lazy: alias_pairs(), value_taking_words(),
    prelude_foreign_types().
  - Seed trap: the current seed reads the typed lines off lib/prelude.warp; without them it compiled `m.keys` of a map
    with a `keys` entry as the entry (core/extension.warp joined_maps: "no field m" at every start of the built
    compiler). So the typed lines stay in lib/prelude.warp and lib/math.warp, ignored by the new compiler, until the next
    seed refresh (card builtins-seed-transition). Reproduce a seed question in seconds, no build:
    `warp ~/dev/bin/warp-seed.wasm -- prog.warp out.wasm <lib>/compiler <lib> ~/dev/bin/lib/runtime.wasm`, then
    `warp out.wasm`.

## Left
- P3 done on branch baked-lib-split-3 (worker warp-baked-lib-split-3): core/prelude_core.warp, the prelude's core
  part (the 16 words the compiler calls, the 12 aliases its code writes); std_module_text("prelude") = core part + lib
  file, the one reader (notes/library_zero_bake.md "core-takes-what-it-uses"); compiler-inputs.sh --baked-files lists
  no lib/ file. Left from it: the dead reaches by name (card note), which still compile ~45 uncalled lib prelude words in.
- After the seed refresh: card builtins-seed-transition (it also deletes lib/prelude.warp's copies of the core part's
  12 alias lines; compiler.warp's include of core/prelude_core.warp can go then too).
- Card the three gcds (lib/math.warp wide_gcd, extensions/big_numbers/exact.warp int_gcd, core/integers.warp gcd) and
  lib/math.warp's bit_and/bit_or beside the runtime's bits_and/bits_or.

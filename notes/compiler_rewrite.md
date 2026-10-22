# The compiler rewrite: plan, state, next (user 2026-10-22)

The user's main goal: the warp compiler written in warp replaces the Rust one, built as notation → kernel → sugar.
Details of the layers, every sugar form and the rule it becomes: notes/layers.md. Rule engine spec: notes/rewrites.md.
Decisions: language-layers, kernel list, alias-mechanism, n-times, runtime-split, name-glue (notes/decisions.md).

## The plan (2026-10-22 noon)

1. **Rust is frozen legacy.** New features, generalizations and cleanups go into warp (lib/prelude.warp,
   lib/compiler/*.warp). Rust changes only to keep the bootstrap building, fix a crash, or `warp fix`/`warp format`.
   A Rust hack warp does differently gets the banner `// ⚠ WARP DOES THIS DIFFERENTLY: … → <lib file>`.
2. **Three layers** (notes/layers.md):
   - *notation*: the reader warp data uses (atoms, keys, groups, comments, operators from one precedence table,
     juxtaposition), no keywords; extended only by table rows and `operator`/alias declarations.
   - *kernel* (user's list): definition `:=`, assignment, call/member/element, `=>` function, `if`, `while` with
     break/continue/return, `class` with variants, `use`, `try`, `global`, `data` (quote), `rewrite`/`operator`, `match`.
   - *sugar*: everything else as `rewrite` rules into the kernel, one module per topic in lib/sugar/*.warp.
3. **Measured at the start:** parser.warp ~50 % notation, ~16 % kernel, ~34 % sugar (1417 lines); 26 of 62 other
   compiler files (2448 lines) are sugar passes; ~3900 sugar lines in all, expected to shrink to a few hundred rule
   lines.
4. **Rewrite engine first** (lib/compiler/rewrites.warp), features F1–F8 named by what the sugar needs; then
   migration by gain: word aliases → rules that work today → F3 group → F2 group → F1 group → notation tables →
   clauses and English definitions.
5. **Process:** new syntax is a table row or declaration, else a rewrite line; a new parser branch or pass only when
   its commit names the missing engine feature. No new passes. One migrator for parser.warp (warp-layers), the pass
   files to the Grok coders, small branches landed express the same day.

## Achieved (state 2026-10-22 evening)

- **Rule engine:** F1 places (statement head, condition, class body, key, value), F2 where-conditions, F3 fresh
  hygienic names + evaluate once, F4 per-item templates (`T for $x in $xs...`), F6 runs of statements, F7 re-rewrite
  to a fixpoint: all on main, with tests (tests/modules/test_rewrite_engine.warp).
- **Kernel decided** by the user (with `match`); `n times` decided as the library function
  `infix times(n: number, b: block)`, no rewrite.
- **First sugar modules:** lib/sugar/arithmetic.warp (floor division `/!`, `/!=`, `div` alias) and
  lib/sugar/methods.warp (list mutation methods, membership): branch layers (sugar-migration-2), landing now.
- **Not rule-ready yet** (found by warp-layers): of_lookups and number_words (twin stages Rust drives), parse_words'
  eval (needs scoped shadowing), test/emit/unit phrases and `n times` (parser grouping).
- **Around it:** emitter.warp split into 14 files under lib/compiler/emitter/ (bb97b383e7); node patterns and
  beautiful types (named patterns `Definition`, `Call`… in lib/compiler/node_patterns.warp); sum is a plain warp fold;
  both lanes (`warp pure` = compiler.warp built by Rust, `warp self` = built by itself) compile and run hello on main.
- **Honest count:** the sugar passes are all still there and parser.warp is about its old size (4272 lines); about
  1–3 % of the ~3900 sugar lines have moved. The engine was the first half of the work; the moving starts now.

## Next

1. **F5 name building, high priority** (user 2026-10-22 evening, card rewrite-templates, decision name-glue):
   `square·$a` → square_a_number, `$i·end`; unblocks for_loops, English `To …:` definitions, accessors per field.
2. **Sugar passes into lib/sugar/** (Grok coders, one pass per branch, express): coalesce, comprehensions, ranges
   (F3); type_tests, filled_lists, grouped_receivers, prefix_comparisons, plain_enums (F2); foreign_forms,
   welcome_forms (F1/F4); definitions' `def`/`fun`. Each deletes its pass file.
3. **parser.warp** (warp-layers): alias-mechanism (word alternatives into the alias table, migration step 1), then the
   remaining sugar branches, then notation tables (word brackets `do … end`, raw-text block words, compound names).
4. **Runtime split** (Grok Warp Code 3, decision runtime-split): core runtime vs lib/compiler/runtime/extended/, GC
   types emitted on demand; then big ints (pure-big-int-power) into extended/.
5. **Open engine work:** F8 text holes (`"…$x…"` patterns), zip over two rest holes (multiple_assignments).
6. **Later:** of_lookups and number_words once Rust no longer drives their twins; self-hosted speed (self-speed:
   interned texts, unboxed ints, typed structs); twins of Rust removed once the first pipeline runs fully in warp; the
   playground resumes when the warp compiler can replace it (paused 2026-10-22).

Measure progress by the sugar lines left: `wc -l lib/compiler/parser.warp` plus the pass files in the notes/layers.md
table, against lib/sugar/*.warp.

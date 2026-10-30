# Core growth audit, 2026-10-29 → 2026-10-30 (card diff-core)

The question (user 2026-10-30): what did the core gain between the fast compiler (~140 s to compile compiler.warp) and
main now (380–420 s), is each piece adequate for the core, and where did library code get pulled into what every
compiler build compiles or bakes. Reading and git only: no builds, so every cost below is an **estimate to be
confirmed by warp-speed's profile**, ranked by how the cost scales on the compiler's own compile (a ~29 k line program,
~1–2 M nodes), not by line count.

## Baseline and measurements

| seed | built from | compile of compiler.warp |
|---|---|---|
| seed-2026-10-29-f7c80528 (03:34Z) | 0b2df6e915 (10-29 04:19, v1.5.2) | ~140 s ← **the fast compiler, the base** |
| seed-2026-10-29-7afa4465 (07:08Z) | 64a443271f (10-29 07:43) | ~230 s |
| seed-2026-10-29-5e25bfb0 (16:25Z) | c4d710d64a (10-29 13:39) | 187 s |
| seed-2026-10-29-bd1ab597 (19:13Z) | 4470f018e5 (10-29 19:57) | ~340 s |
| seed-2026-10-30-3efd5391 (22:09Z) | main of 10-30 | 380–420 s |

The seeds themselves (wasm-tools on the release assets):

| | functions | code | data (baked lib) |
|---|---|---|---|
| f7c80528 (fast) | 3468 | 1.41 MB | 1.44 MB |
| 5e25bfb0 | 3617 | 1.52 MB | 1.69 MB |
| now | 3969 (+14 %) | 1.61 MB (+14 %) | 2.06 MB (+43 %) |

Source compiled into the compiler (core/ without native/): 26 564 → 28 906 lines (+8.8 %), plus lib/rule_words.warp
and lib/wgsl.warp (1 157 lines, +4.4 %), which the compiler `use`s. Bundled lib/ (every .warp of lib/ and
core/extensions, parsed and baked at each compiler build): 415 KB → 528 KB (+27 %; wgsl.warp 48 KB, rule_words.warp
20 KB, sound +9 KB, json +7.5 KB, reals +4 KB, storage +4 KB).

**So size explains maybe +15–25 % of compile time, not ×2.9.** The rest has to be super-linear or whole-program work
added per compile: the top of this list. The timing jumps line up with it: 140→230 s holds broadcasting's
whole-program `is vector` fixpoint (10-29 05:25); 187→340 s holds called_only (17:38), captures_assigned_later (17:12),
the baked rule tables (17:21–17:44), time literals and exact roots as whole-program scans.

Ruled out: int arithmetic. The fast seed called `checked_*` 3 591 times; now it is 3 825 plain `i64.add` and 360
`grown_*` calls (07daec5cfe, b458fc1d71): the compiler's own code runs *faster* there. Broadcasting rewrote no call of
the compiler itself (no `call $each_any` in the seed). The native driver, warp.warp, tests.warp (core/native/, +630
lines) are not part of compiler.warp: no compile cost.

## Top 5: move out / undo

1. **Broadcasting's `$x is vector` — a whole-program fixpoint** (rewrites.warp `vectors_of`, 132efe4f79 + 59e832c38e).
   `$f($x)` has no literal word, so lib/sugar/broadcasting.warp's rule is tried at every one-argument call; its first
   condition `$x is vector` makes vectors_of materialise `nodes_in` of every statement and iterate *all* nodes until no
   name grows (rounds × all nodes × may_be_list). Fix: test the cheap `$f takes number` first (rule condition order, or
   the engine ordering conditions by cost); make `is vector` demand-driven per name with a memo instead of a fixpoint
   over the program. Expected: the biggest single gain (the 140→230 s jump).
2. **called_only is quadratic** (prelude_words.warp, 1f6734dee8, ecfa29632f). For each called word it scans every
   statement for `declared_name == word`, and `pending = pending[1 ..]` copies the queue on every step (~10⁵ words
   for the compiler → ~10⁹ item copies). Belongs in core; write it linearly: a name → statement map once, a stack or an
   index instead of slicing. Expected: large.
3. **captures_assigned_later is quadratic per scope** (lambdas.warp, 6732010d74). For every statement it recomputes
   `assigned_names` (a nodes_in) of *all later* statements, even in scopes without a lambda — the main level of the
   compiler has thousands of statements. Fix: return [] when the scope has no lambda; build the later-assigned sets
   once, from the end. Expected: medium–large.
4. **The library baked in every compiler build** (uses.warp module_text, parser.warp LIBRARY_FILES/LIBRARY_PARSES,
   prelude_words.warp LIBRARY_SLICES/SLICE_PARSES, rewrites.warp LIBRARY_RULES/STD_RULE_NEEDS, parser.warp
   BLOCK_HEADS; d9d4131f70, 3b50f73753, ce88d05a97, 853a27c0a6, 568899f9f5). Each compiler compile parses all 528 KB of
   lib/ ~3 times (each file, each slice again, the rules), and `baked-current` in `warp rebuild` computes all tables a
   second time. The tables depend on lib/ only, never on the compiler: compile the compiler with empty tables and
   always `rebake` them (the mechanism exists since d06d666bda), from a store keyed by the lib/ hash (as the new
   machine-wide compiler store is). Expected: medium–large, and lib growth stops costing compiler builds.
5. **lib/wgsl.warp and lib/rule_words.warp compiled into the compiler** (`use rule_words` in core/passes/rewrites.warp,
   8bcf87194d; rule_words `use wgsl`, 864ec28f55, 03def76ca5): 65 wgsl_* and ~48 rule-word functions, ~23 % of the
   ~500 functions the compiler gained. A WGSL shader translator is a library, not the compiler. Fix: rule words a
   module needs live with that module (web's served/route_*, frames' framed, signals' signaled, wgsl's
   wgsl_painting/gpu_*), built once as that module's prebuilt wasm and called by the compiler only when a rule of that
   module fires; the general reflection words (function_definition, body_of, doc_of, dir_words …) become
   core/extensions/reflection.warp. Expected: ~5–8 % (proportional to code), more as shaders grow.

## All features, sorted by expected gain

Lines = lines added in core/ + the baked libs (prelude, aliases, math, rule_words) by the feature's commits.
Cost: ●●● whole-program super-linear, ●● extra whole-program walks per compile, ● per-node constant or size only,
○ no cost to compiling the compiler.

| # | feature (commits, card) | lines | verdict | reason | cost |
|---|---|---|---|---|---|
| 1 | broadcasting P50, `is vector` (132efe4f79, 347be786a9, 59e832c38e, e9b5da3b59, 030223e0bb, acd76019ec, 98f81b2a53, 19dbf5e34f, df99491ec2) | ~190 | do differently | demand-driven vector facts, cheap condition first (above #1) | ●●● |
| 2 | called_only, drop uncalled brought functions (1f6734dee8, ecfa29632f) | ~60 | core, rewrite linear | quadratic scan + queue slicing (above #2) | ●●● |
| 3 | captured variable is the same variable (6732010d74, decision captured-same-variable) | 14 | core, rewrite linear | O(N²) nodes_in per scope (above #3) | ●●● |
| 4 | baked library tables, block heads, extensions bundled, rebake (d9d4131f70, 3b50f73753, ce88d05a97, 716659e829, e2b1898b0f, 853a27c0a6, 568899f9f5, d06d666bda, 0947e21bef, 845734129f, 8593f622d4) | ~290 | move out of the compile | lib-only data, build once per lib hash and rebake (above #4) | ●● |
| 5 | rule words + WGSL shaders + reflection words (8bcf87194d computed-hole, 864ec28f55, f26d7efe96, 69ebb9f6c6, 4ff8a8c3ab, 351ad7012b, a3dc71d4a7, 15550e4139, 5bbcb7e0b7, 22272c86f9, 71a0810106, 71ee56853f, c429914988, 2539bf747c, 687b40de72, ec4b466cca, b690562b9e) | 75 core + 1 157 lib compiled in | lib module (wgsl), core/extensions (reflection), computed holes stay | compiler code size; vocabulary of modules in the compiler (above #5) | ● (size) |
| 6 | runtime shaking + host import drop (948b3582e5 runtime-shaking, 6fe0012423, b3d282e178, 9b75aecc00, 421e82222a, b73234269e, c06d6f5264, 71ca3a34e7) | ~445 | core, do more generally | `relocates()` is now true for every compile with `shake_runtime` (compiler.warp: no library runtime, no separate output; to confirm that `warp rebuild` takes this path): all code emitted padded and marked, then every body re-encoded (moved_body) to save nothing for a compiler that reaches the whole runtime. Number runtime functions by reach *before* emitting instead of rewriting bytes after; skip when the reach is most of the runtime | ●● (byte lists over 1.6 MB) |
| 7 | time literals as values, time fields, time arithmetic (1abec60c4b, 6fb917b1a0, 4ec7932865, d5c57b0666, 0d7f1a2ebe) | ~175 | do more generally | `time_literal_words` adds 1–2 nodes_in of the whole program and of every reached slice (lowering_words), called again in called_only and emitted_modules; walk.warp and emitter/times.warp name time_field_of, time_operation, time_arithmetic: domain names in core. One mechanism "a literal kind a library declares brings its module" | ●● |
| 8 | exact roots, as-promotes, float contagion, converted types (baef255e49, ebbac61010, 6afc9e9c49, 75c2701885, 7e7f052cbc) | ~130 | do more generally | root_words and converted_types are two more whole-program nodes_in, root_words also per slice. Fold #7 and #8 into one walk collecting "words the program's forms need" from a table | ●● |
| 9 | GPU lists [f32] (a92ecbe302, 1af7f9d28e, decision gpu-bound-floats) + core/runtime/gpu_lists.warp | ~125 | lib module | a `where program` rule in **the prelude** runs gpu_lists_packed (a full walk) wherever the prelude's rules are kept (own_rules_applied keeps a module's program rules with any of its others: likely the compiler's compile too, to confirm); GPU words in core/runtime are vocabulary. Move the rule and the words to a gpu module brought by gpu_compute; generally: a program rule is gated by its own needed words | ●● |
| 10 | result-type overloads, defaulted variants, definers by argument count (c70042ab69, bfcca8664d, ee1ce7f4d8) | ~160 | core, share the walk | overloaded_results and definitions_named (twice, via variant_least and defined_types) each walk nodes_in(program); one definitions table per pass; definers_taking and the implicit-use diagnosis add a nodes_in per ambiguous word | ●● |
| 11 | general .op / op= forms, operator holes, √ ∛ ‖ as functions (fe1dcbea4f, 0cca448032, 749ad50f42, 40be1f840a, 119bdffd63, e846619793, 642ef81537) | ~200, −128 | core, keep | a mechanism replacing tables (−128 lines in math/core); names_within now calls operated_within per symbol: cheap | ● |
| 12 | big ints, big ratios, exact decimals past 18 places, nat/int (7134fa3460, 34e0dbc0ce, a0bdb0602e, b458fc1d71, 03d27e42bc, 07daec5cfe, 6630a61f2c, 3db6be575f) | ~435 | core/runtime/extended, keep | runtime.wasm words, built once per rebuild and shaken per program; made the compiler's own arithmetic cheaper (checked_* calls gone) | ○ / gain |
| 13 | int types `type T = int` (bf79243852, 196d047fa5) — two new core/passes files | ~130 | core/extensions file | a new pass inside definitions.warp without the user's yes; one statements scan when no int type: cheap | ● |
| 14 | signals, on set / whenever / once / derived (13a1688770, 34e686a036, 1336c4817f, f93773c3db) | ~90 core+rule words | lib, keep — but rule words with the module | library feature; its `signaled` program rule is only kept when signals is used; its rule words are compiled into the compiler (#5) | ● |
| 15 | linked modules: variants, method clash (0ab4ec3466, ee77316666, a9a9b24197, c311aa20c5) | ~50 | drop once bind-linked lands | LINKED_METHOD_CLASH is a workaround its own comment says (card bind-linked) | ○ |
| 16 | shown result as warp code (bb71e75f6a), dir, f.doc/signature/definition | ~115 | keep (show in core), reflection → extensions | | ● |
| 17 | phrase and spacing readings: commas, spaced values, glued negative, trailing percent, `sq 2 == 4` refused, labels, bang mutators (bcbc9b81e9, 20e9bb432c, 26c87d5f37, 4bda6e575d, 1990dab0c1, 6f398aaf32, 950e19d861, 4954b5b0aa, 7fe26883c2, f08cddfbb5, 2a31fec7a2, 44211f506f, 66ba474e94, d78a753c50, b57200d57f) | ~210 | core (parser/phrases), keep | decided syntax; linear in the parse. `count of membership refused` (696e97b27e, 4ede3386b3, abca208f08) is an esoteric refusal path → a lint in core/extensions | ● |
| 18 | parse_code, rebake, --no-hints, version stamp, GC heap sizes, test timings (90cd729fd6, 31e71e0739, c3cbd5f886, 528a0c4095, be326fba75, b5d4712fad) | ~250 | core commands, keep | commands, run on demand | ○ |
| 19 | native driver and CLI: repl-line, window/display host words, crash lines, freeing stores (998445f595, 44c16f1f0e, cd065c751e, a24b30fa50, 51c1081b51, 7ba2204053, 63203b7423) | ~350 | window/display host words → lib host words | not compiled into compiler.warp; `window.width`/`display.dpi` answered by the driver are host vocabulary (notes/roadmap_native_harness.md says lib) | ○ |
| 20 | perf work that pays back (69f436e8bc, 4506f15aab, 54a8afa00a, 71ca3a34e7, aca10be3a9 shared subtrees, 2ba0a38e3e shared small ints, 1fb3f17620 linear show/join) | ~110 | keep | gains | gain |
| 21 | Rust harness gone (d115331edf) | 169/−169 | keep | comments only in core | ○ |

## Library code pulled into each compiler build

1. **Compiled into warp.wasm**: lib/rule_words.warp (`use rule_words` in core/passes/rewrites.warp) and through it
   lib/wgsl.warp — 1 157 lines, ~113 functions. Nothing a program does needs them inside the compiler except at
   compile time of a program using paint/shader_of/routes/signals/frames/dir/f.doc.
2. **Baked as data at every compiler compile**: every .warp of lib/ (not compiler/, data/) and core/extensions —
   texts (LIBRARY_FILES), whole parses (LIBRARY_PARSES), per-slice parses (SLICE_PARSES), slices, rules
   (LIBRARY_RULES), rule needs (STD_RULE_NEEDS), block heads (BLOCK_HEADS). Any new lib file (wgsl, storage,
   signals, frames, json …) costs every compiler build three parses, plus once more for `baked-current`.
3. **Run on the compiler's own source as a program**: the prelude's rules (gpu_lists_packed's program rule, paint
   and shader rules), lib/sugar/broadcasting.warp (`is vector`), the prelude words the compiler reaches and their
   slices (with_reached, per slice now 3–4 walks instead of 1).

## Not measured, to measure

warp-speed's profile should time, on one compiler compile: vectors_of (and how many rounds), called_only,
captures_assigned_later, the module_text bakes (library_parses, slice_parses, library_rules, std_rule_needs,
block_heads), gpu_lists_packed, the shaking (shaking_of, moved_body, drop_imports) and each `nodes_in(program)` call
site of prelude_words.warp. The ranking above is by algorithmic shape; a profile may reorder #4–#9.

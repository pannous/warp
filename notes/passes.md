# Passes: inventory, overlaps, merges (card passes-audit, 2026-10-21)

Rule (notes/agents/common.md "No new passes", user 2026-10-21): a feature extends or generalizes an existing pass; a
new pass needs a reason no existing one can carry, named in the commit.

## Inventory

src/pipeline.rs `lower_with_sealed_quotes` runs, in order:

| stage | entries | what |
|---|---|---|
| before the lists | 5 | `rewrite p => t` rules, for-loop headers, typed block arguments, `data e` sealing, data items |
| checks | 5 | operator-word functions, upcast fields, self-typed fields, accessibility warnings, source warnings |
| SUGAR_PASSES | 48 | written forms: interpolation, welcome forms, `of`, phrases, markup, patterns, serve, lambdas, reflection words, pipes |
| DECLARATION_PASSES | 30 | classes, units, type aliases and parameters, traits, tables, foreign modules, aliases, routes |
| MODULE_PASSES | 8 | `modules::resolve` and the passes that read the used modules' words again |
| SIGNAL_PASSES | 25 | stored/system values, signals, events, keys, elements, components, pages |
| FUNCTION_PASSES | 34 | declarations, parameters, blocks, mutation, getters, named arguments, tuples |
| between | 7 | kind changes, uncertain orderings, function equality, time, static units, exact reals |
| MEANING_PASSES | 35 | types and traits, broadcasting, overloads, lambdas, closures, library words, switch, memoization |
| after | 10 | effects, late binding, main globals, diagnose, lint, data scope, declarations, inlining, self sums |

About 200 steps; src/lowering is 57k lines, src/analyzer 9k. A used module's source runs SUGAR + DECLARATION
(`lower_module_source`) minus the program-only passes. The warp twins: lib/compiler/passes.warp `passes` (8 twinned
with Rust through src/compiler/self_hosted.rs TWINNED_PASSES) and `pure_passes` (33, the pure lane).

Cost: lowering is ~5 ms of a 115 ms hello run (`WARP_PHASE_TIMES=1`); the payoff of merging is clarity and fewer
ordering constraints, not speed.

## Overlaps, ranked by payoff

1. **Sugar phrases as rewrite rules** (largest). About 15 passes turn one written phrase into a canonical call:
   of_lookups, member_phrases, phrase_words, times_phrases, negated_calls, number_words, word_operators (2),
   std_aliases, rust_paths, welcome_forms (4), preposition_calls. `rewrite p => t` (modules/rewrites.rs) already
   carries such rules from lib/*.warp. Each pass whose rule is a pattern with no program-wide fact could become a
   rule in a lib file; the Rust passes then shrink to the cases needing bindings (a name the program defines wins).
   Card sugar-as-rewrites. First move: `add x to list` (phrase_words) is `rewrite (add $x to $list: symbol) =>
   $list.add($x)` in lib/english.warp. What keeps the others in Rust, each an extension of rewrites.rs:
   - no notes: a rule cannot say the got-it note of the alias rule (diagnostic::note_alias), which std_aliases,
     rust_paths, welcome_forms and word_operators give;
   - shadowing is program-wide (a standard rule drops when the program binds its word anywhere), the passes shadow
     per scope (program_names::rewritten_where_free: word_operators, number_words);
   - rules run first, before every pass: member_phrases must follow the markup passes, times_phrases the parser's
     loop shape;
   - patterns: none may start with a hard keyword (`do`), the parser reads `add $x to $l => …` as arithmetic
     (grouping parens fix it), and conditions are only typed holes and `where $c = shape` (member_phrases needs "no
     operator word among the arguments").
   With notes and scoped shadowing in rules (card rewrite-notes-scopes), word_operators, std_aliases and number_words
   can follow.

   Expressed in existing warp syntax (user 2026-10-21: "can they be expressed with given warp syntax, so as a lib"):
   - plain definitions, as lib/PHP.warp and lib/R.warp already do (`array_sum(xs) := sum(xs)`): foreign words that
     are ordinary calls. Missing: the got-it note, and a module-qualified name (`json.loads`) can't be defined that way.
   - `alias X = Y` exists in lib/collections.warp for class names (with a note) but not for words: in a program
     `alias root = sqrt` fails ("root is a keyword"). Proposed generalization: `alias PATTERN => TEMPLATE` is a
     rewrite rule that says the alias note, so the tables of std_aliases (`alias re.sub($p, $r, $t) =>
     replace_all($t, $p, $r)`, its module brought by `use regex` as a fired rule brings it), rust_paths (`alias
     Vec::new() => []`, `alias i32::MAX => 2147483647`), word_operators (`alias root $x => sqrt($x)`) and the
     welcome forms move to lib/python.warp, lib/rust.warp, lib/js.warp beside PHP.warp and R.warp. `rewrite` stays
     for canonical warp phrases (english.warp), `alias` for other ecosystems' words.
   - what stays a pass: forms the parser shapes (`(p).a`, `xs .* 2`, `"\(x)"`, `x | f`, `n times {…}`), forms
     needing bindings (of_lookups' "a word the program defines", grouped_receivers' bound variables, negated_calls'
     functions) and keyword-headed forms (`do`).
2. **Reflection: ten entries for one idea.** reflection::lower (twice), lower_objects, lower_component_words,
   lower_module_words, lower_unknown_dir, meta_values::lower and with_table, word_help::lower and lower_named_words,
   doc_strings. Decision reflection-plain says `meta(x)` is the one core builtin: every other word (`dir`, `.fields`,
   `.methods`, `.exports`, `help`) is a view of the meta table. One table built once after resolve, the words as
   views of it, removes the second reflection::lower and most entries. Card reflection-one-table.
3. **Signals: 25 entries, three families of listener.** system_values (2), signal_values (3), key_names (2),
   element_events (2), event_signals, variable_signals, system_signals, fetch_signals, component_state, stored_values,
   undo_history. Each `on X {…}` form makes a handler and a subscription in its own pass. A single listener lowering
   with a table of sources (timer, key, element event, variable change, system value, fetch) would carry them.
   Card signals-one-listener-pass.
4. **Checks scattered as passes.** match_exhaustive, unit_names, unresolved_blocks, discarded_values sit in the pass
   lists as identity passes that report; ten more checks are inline in lower_with_sealed_quotes, each with its own
   `if let Some(x) = check(&node) { return Err(..) }`. One `CHECKS` list per stage point (as written, after source
   passes, after meaning passes), run by one loop, takes them out of the pass lists. Card checks-list.
5. **Units and intervals: 11 entries.** remember_field_unit_words, lower_unit_words, lower_field_tolerances,
   lower_run_time_tolerances, lower_sleep_durations, lower_quantity_comparisons, static_units::lower,
   partial_comparisons, uncertain (lower_certainty, check_orderings, lower_interval_parameters). The four adjacent
   DECLARATION entries are one units pass in four steps. Card units-passes.
6. **Broadcasting: 5 entries** (element_wise_operators, scalar_element_wise, prefix_calls, lower,
   several_arguments), interleaved with arity overloads. Worth one look whether lower_scalar_element_wise and
   lower_prefix_calls can join lower. Card broadcasting-passes.
7. **Passes run twice.** getters::lower (modules' getters wait for the program), reflection::lower (again after
   resolve), blocks::seal_quoted (after parse_word and blocks make new `data e`). Lowering getters with the module and
   sealing where data is made would make each run once. Part of the cards above; getters: card getters-once.
8. **Whole-program facts recomputed per pass.** `Bindings::of` 27 call sites, `function_definitions` 14,
   `ProgramNames::of` 12, `class_names` 9, `InstanceTypes::of` 9. Each pass rescans the program. Low payoff given
   the timings; worth it only together with a shared facts object the passes ask. Card Later program-facts.
9. **Adjacent entries of one file** that are one pass in steps: welcome_forms (3 adjacent: match arms, bare
   returns, arrow result types; twinned separately in warp), lambdas (infix iterations, member lambdas),
   declarations (c functions, bare declarations, spaced definitions), meta_values (lower, with_table). Composing them
   into one entry each is cosmetic; do it when the file is touched. The twinned ones need their warp twin merged in
   the same commit.

10. **Every pass walks the whole program; none is picked by keyword.** The pipeline folds each list over the tree,
    so all ~200 steps run on every program. About 40 passes open with a guard (`mentions_any(&[OF_WORD])`, a
    `visit` pre-scan) and return the node unchanged when their word is absent, but the guard is itself a full walk.
    Generalization: each pass declares its trigger (words, or a node shape such as `(p).a` or `x | f`); the
    program's symbol set is collected once per phase (again after `modules::resolve` and after passes that generate
    code) and a pass whose trigger is absent is skipped. Turns ~40 hand-written guards into data and makes "which
    passes does this program hit" visible (`WARP_PHASE_TIMES`). Card pass-triggers.

## Done (branch passes-audit)

- type_parameters: `lower` and `lower_type_variables` walked the same definition heads with two predicates; one walk
  now takes both, and the extra `declares_type_variable` scan is gone.
- serve: `with_default_routes` (hosted) and `with_page_routes` (run) were two passes, each a no-op in the other's mode;
  one `with_implied_routes` picks by the compile mode.
- library_words: `lower_byte_counts` is the last step of `library_words::lower`, which wrote what it reads.

## Keyword-rewrite table: measured, parked (user 2026-10-22)
Merging local sugar passes into one walk (src/lowering/words/keyword_rewrites.rs, SUGAR_PHRASES) saves next to
nothing: in a 52 s cold compile of passes.warp and its uses (WARP_PHASE_TIMES, empty WARP_MODULE_CACHE, main's build)
the four passes it replaced took 59 ms together (of_lookups 45.5, grouped_receivers 5, element-wise 4.7, arrow
results 4), about 0.1 %; a front-lane twin stage costs a few ms. The time is in resolve uses (8 s), the meaning passes
(9.5 s), emit (6.4 s) and diagnose (4 s). The table gives one shape, not speed, and its trigger rule ("own words")
grows with each entry (nested phrases had to join for `sum (of xs)`, card fixnow-of-phrase-as-a-value). Parked:
card arrow-result-entry (branch arrow-result-entry); per-statement passes: card apply-lowering. Speed work belongs to
the phases above.

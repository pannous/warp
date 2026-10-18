# Self-hosting: dropped forms and deviations, per component

Where to find things:
- Rules: the two kinds of entry below; the rule itself is in notes/self_hosting.md ("matching Rust is a check, not
  the goal", "success-driven").
- Parser (lib/compiler/parser.warp): dropped forms → "Parser: library data DSLs", "Parser: forms to restore later";
  tree shapes → "Parser: deviations".
- Passes (lib/compiler/*.warp passes): dropped forms → "Passes: dropped forms"; per-pass plan → "Passes: SUGAR_PASSES
  triage"; target order → "Passes: target pipeline"; "Passes: deviations".
- Analyzer (lib/compiler/analyzer.warp): "Analyzer: deviations".
- Emitter (lib/compiler/emitter.warp): "Emitter: deviations".

Two kinds of entry:
- Dropped form: a form the warp part leaves out (the cleanest, most minimal compiler, not a copy of the Rust one),
  with the sources that show it, why, and how to bring it back. The harnesses skip the named sources:
  tests/parser/self_parser_dropped.txt (path, tab, form) for the parser survey `parser_differences`
  (tests/parser/test_self_parser_survey.rs), and the `dropped` list of tests/common/twins.rs pass_differential, which
  refuses a dropped source this file does not name. A pass that looks like bloat as a whole goes to the Supervisor
  instead of being ported. `probes/self_hosting/parser/sort_differences.py` sorts remaining parser differences.
- Deviation: where the warp part keeps a cleaner result than its Rust twin, with an example and the card or question
  for the Rust side. The differentials read them as expected.

## Parser: library data DSLs
Code and data are one: once the parser reads plain warp data correctly, these need no syntax of their own.

| Form | Example | What it takes |
|------|---------|---------------|
| Music `go {…}` blocks | `go { c4 e4 g4 }` | a sound library function taking data, `tune d:data := play d`, so `tune { c4 e4 g4 }` plays plain data; note names are constants of the music/sound library (card music-data-dsl) |
| CSS in style blocks | `style { td#x { color: red } }` | `style` reads the block as data: selectors as keys, declarations as pairs (card style-data-dsl) |

## Parser: forms to restore later
| Form | Example | Why dropped | What it would take |
|------|---------|-------------|--------------------|
| Other languages' syntax | `println!("{}", x)`, `(x for x in xs)`, `yield from g()`, `xs.each { \|x\| … }` | compatibility reading, not warp | port warp_parser/rust_macros.rs, the Python generator and Ruby block forms (definitions/foreign.rs) |
| Phrase definitions | `to get value of coin in cents: …` | experimental natural-language heads | port the phrase head reading (phrase_definitions in warp_parser) |
| Shader holes in WGSL text | `shader "…{x}…"` with `shader_holes` | experimental GPU text | port the hole scan for shader texts |
| Error nodes for malformed input | `y = [1 2` → "`]` is missing" | come back with the diagnostics port | the parser's error nodes and their texts |

`$a` pattern holes are kept (the pattern roadmap in notes/matching.md is built on them).

## Parser: deviations
tests/common/parser_deviations.rs rewrites the Rust tree to the clean shape, one function per entry; when Rust
changes (card in the last column), its function goes.

| Key | Form | Rust | parser.warp | Card |
|-----|------|------|-------------|------|
| definition-head | `f(x, y) { x }` | `((f (x y)) {x})`: the parameters a nested group | `((f x y) {x})`: the head a call | rust-definition-head |
| bare-return | `return` | the phrase `[return ø]` | the word `return` | rust-bare-return |
| colon-body | `for 1..3: print it` | the body wrapped as `{;}` | the body bare, as in `for i in xs: x` (`3 times: x` keeps `{;}`: a block runs, `3 times "ab"` repeats) | rust-colon-body |

Fixed in both parsers instead: hex digit separators `0xFF_FF`; a wrong closer `(1 + 2]` and a stray top-level `]`
are error nodes; `3 times: print it` runs the whole rest of the line.

## Passes: dropped forms
Per form: the form, the sources showing it, why it is left out, how to bring it back.
- sql and sh templates: `sql "SELECT … $name"`, `exec sh "ls"` as parameterized calls (src/text/injection.rs
  lower_templates, run first by interpolation.rs lower_program). Sources: probes/docs_sync/sandbox_exec.warp,
  probes/sandbox/escape.warp, probes/sandbox/exec.warp, samples/system/crontab.warp. Left out for now: a library
  feature, not text building. Back: port lower_templates in front of lib/compiler/interpolation.warp.
- Rust Option forms: `if let Some(v) = o { A } else { B }`, `match o { Some(v) => A, None => B }`, `o.unwrap_or(d)`,
  `o.expect(m)`, `o.is_some()`. Sources: probes/rust_reading/option.warp. Left out: warp's own forms say it
  (`if o != ø`, `o ?? d`, `o ?? raise(m)`). Back: port src/lowering/welcoming/rust_options.rs as a pass before the
  match arms.
- `set x to 5`, `set width of p to 12` (decision setter-calls: `p.width = 12` with a style hint). Sources:
  probes/english_setter.warp. Comes back as a library code template, not a pass (card set-set). Back meanwhile: port
  welcome_forms.rs lower_set_phrases after `a of b`.
- `print words in t` (the walk `for part in t.words {print part}`) and `byte count of t` (the count in a unit,
  `t.bytes`) in of_lookups (decision sugar-triage). Sources: none in today's corpus differs. Back: port of_lookups.rs
  print_walk and the text_unit branch of looked_up.
- `(function f).dump` in grouped_receivers: a function reference in parentheses keeps its group. The bridge's plain
  view drops the function-reference mark (an attribute layer), so the twin sees `(f)` and ungroups it when f is a
  loop variable. Sources: probes/joinable_error.warp. Back: keep the function-reference mark in plain views (twins.rs
  plain, walk.warp plain). The self-hosted bridge keeps attribute layers since card suite-sort-bang (walk.warp
  bridged, node_views handled_plain_view); the twin comparisons still use plain.

## Passes: SUGAR_PASSES triage
Sources changed = probes and samples whose program the pass changes (tests/parser/test_sugar_survey.rs,
`tests/queue.sh -- sugar_pass_survey --ignored`); a check (no rewrite) changes none. Lines = the Rust file.
Plan: keep = port as a warp pass; library = comes back as library code once modules can extend the compiler, not
ported; drop = left out; merge = one warp pass with its neighbour.

| # | Pass | Lines | Sources changed | Plan |
|---|------|-------|-----------------|------|
| 0 | interpolation | 104 | 49 | keep (ported) |
| 1 | rust_options | 160 | 1 | drop (decided) |
| 2 | match arms | welcome_forms | 15 | keep (ported) |
| 3 | arrow result types | welcome_forms | 0 | keep (ported, merged into welcome_forms.warp) |
| 4 | of_lookups | 241 | 22 | keep `a of b` and `in`/`from` after a selector word; drop print walks, unit counts (decided, sugar-triage) |
| 5 | set phrases | welcome_forms | 1 | drop (decided, card set-set) |
| 6 | grouped_receivers | 69 | 1 | keep |
| 7 | module_properties | 467 | 3 | library (graphics: window.width; decided) |
| 8 | times_phrases | 57 | 26 | keep (ported) |
| 9 | broadcasting | 1410 | 11 | keep, minimal port |
| 10 | shader_holes | 250 | 11 | library (gpu; decided) |
| 11 | conversions | 93 | 0 | keep |
| 12 | foreign_lines | 78 | 0 | keep |
| 13 | inline_unions | 30 | 0 | keep |
| 14 | scoped_handlers | 38 | 1 | drop (only the error for a form dropped earlier; decided) |
| 15 | component_worlds | 285 | 2 | library (wasm build; decided) |
| 16 | channel_words | 189 | 3 | library (tasks; decided) |
| 17, 18, 45 | markup_tags (form routes, html attributes, tags) | 357 | 1, 1, 14 | library (markup), one pass (decided) |
| 19 | soft_keywords | 239 | 3 | keep |
| 20 | parameter_patterns | 68 | 2 | keep |
| 21 | index_positions | 284 | 0 | keep (check) |
| 22 | type_name_matching prefixed declarations | 336 | 3 | keep |
| 23 | test_blocks | 118 | 10 | keep |
| 24, 25 | plain_enums, sum_variants | 168, 135 | 5, 3 | keep, merge |
| 26 | match_exhaustive | 185 | 0 | keep (check) |
| 27 | late_binding global split | 603 | 0 | keep |
| 28, 29, 30, 39 | serve (default routes, page routes, path parameters, serve) | 1123 | 0, 12, 8, 23 | library (web), one pass (decided) |
| 31 | named_patterns | 224 | 2 | keep |
| 32 | switch unit names | switch.rs | 0 | keep (check) |
| 33 | tree_walks | 203 | 7 | keep |
| 34 | quasi_quote | 102 | 6 | keep |
| 35 | comprehensions where | 554 | 21 | keep |
| 36 | music_words live loops | 636 | 1 | library (sound; decided) |
| 37 | go_blocks | 259 | 24 | keep (decided) |
| 38 | references | 193 | 0 | keep |
| 40 | style_rules | 220 | 6 | library (markup; decided) |
| 41, 42 | lambdas infix iterations, member lambdas | lambdas.rs | 5, 0 | keep, merge |
| 43, 44, 46 | glued_attributes, select_options, transitions | 43, 81, 242 | 1, 1, 3 | library (markup; decided) |
| 47 | member_phrases | 31 | 1 | keep |
| 48, 50, 51 | introspection, word_help, reflection | 54, 217, 1022 | 3, 1, 5 | library (help and reflection; decided) |
| 49 | doc_strings | 149 | 0 | keep |
| 52 | word_operators | 116 | 0 | keep |
| 53 | pipes | 337 | 4 | keep |

## Passes: target pipeline
Rust passes that only repair an earlier stage go once that stage writes the clean form (also the Rust cleanup's
plan, notes/rust_cleanup.md):
- grouped_receivers exists because the parser gives the call `f()` and the group `(f)` one form; it then guesses
  from program-wide bindings (not scopes). With parser-application (`f()` a call, `(f)` a group) the pass goes.
- of_lookups repairs two parser shapes: `type of x` read as a declaration head (type_of_called) and `x = sum of xs`
  grouped as `sum (of xs)` (as_one_phrase). With the parser-keyword-table row for `of` both repairs go; what stays is
  one rule: `a of b` and `<selector> in|from b` are `b.a`. Its selector check is program-wide (a `words = …` anywhere
  turns `words in t` back into membership); a scope pass decides per use.
- Word phrases into one pass: of_lookups, member_phrases, times_phrases and number_words each scan word phrases
  for an English form; one phrase pass with a table (word → rewrite) replaces four walks, in the
  library-word-tables format.
- Checks are no lowering: match_exhaustive, switch unit names, index_positions (check) and check_unit_names belong to
  the analyzer after the lowering.
- Already merged in the triage table: plain_enums + sum_variants, the two lambda passes, markup and serve groups.

Target order: parser (clean tree) → desugar table pass → scope pass (calls, bindings) → phrase pass → structural
passes (test_blocks, enums, comprehensions, go_blocks, references, lambdas, pipes, quasi_quote, tree_walks) →
analyzer checks.

## Passes: deviations
- Names come from lib/compiler/scope.warp (one walk for every binding form), not from three per-pass walks
  (of_lookups defined_names, grouped_receivers bound_variables, number_words names_given). Cleaner on one corpus
  source: `fn one() {…}` (probes/algo/queens/p8_plus_eq_call.warp) keeps the function's name, Rust's number_words
  makes it `fn 1() {…}`, since its names_given misses `fn`/`def` definitions (card rust-renames). The pipeline
  differential skips that source (tests/parser/test_self_passes.rs SCOPE_DEVIATIONS).

## Analyzer: deviations
One general flow inference instead of Rust's heuristics (card lib-compiler); the twin (tests/common/twins.rs
analyzer_differential) accepts a slot when warp's kind is Rust's or a more general one (covers: empty ⊒ all,
data ⊒ int/float), and the real measure is the suite through the warp compiler. Deviations kept:
- `round_to`: warp int where Rust says float (accepted, card inferred-kinds).
- Recursive results (sources q3–q8, guarded_number): Rust infers node, warp keeps list. The flow join is the more
  precise answer.
- `g := it#2`: Rust adds an extra `g: text` variable; warp reads `g` as a function of it, no variable.
- closure_call_N helpers: Rust's text lists them, warp leaves them out (lowering artefacts, not program slots).
- Builtin calls read their kind off `call_kinds` (lib/compiler/words.warp, checked against Rust's analyzer by
  test_self_words call_kinds_are_rusts); a word nothing defines or tables (a function value, a host word) is any
  value, where Rust says int. `length` is left out of the table (card map-key-length).
- Element types flow through function results (`r = f(); r#1` is int when f gives a list of ints), where Rust says
  any value: probes/algo/dijkstra/12c_implicit_return_list.warp, probes/algo/sorting/69_return_list_noreturn.warp,
  probes/module_scope/mytext.warp and four more left the pinned list for it.

## Emitter: deviations
- Value layout: typed values inside and a boxed $Node only at the edges; texts as GC byte arrays, lists as vectors
  (notes/self_hosting.md "Value layout"). Rust boxes nearly every value as a $Node, keeps texts in linear memory as
  (ptr, len) $String and lists as cons cells. Example: `xs#500` walks 500 cells in Rust's module, one array.get in
  warp's. Same program results. Rust side: card rust-emitter.
- A declared element type on a lax alias: `names: texts = ["hi"]; ys = names; ys.add(420)` makes Rust's module fail
  ("list cannot hold this item"); warp's gives ["hi" 420], because the lowered view the emitter reads carries no
  declaration. Sources: probes/variance/runtime_lax_alias.warp, probes/variance/runtime_unannotated_param.warp. Back: the lowered view keeps the declared element type
  of a list, the emitter checks adds against it.

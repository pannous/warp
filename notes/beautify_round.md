# Beautify round (card beautify-round, Later; start after the hello-world milestone)

Audit 2026-10-20 by warp-reviewer: the 95 non-merge commits since 2026-10-20 00:00 touching lib/**/*.warp, plus the
open branches pure-parser-rest, pure-field-slice, pure-field-defaults, pure-named-arguments, unicode-glyphs.
Line numbers are origin/main at bac59bd016 unless a branch is named. Every form proposed below was tried with
/Users/me/dev/bin/warp, plain and with WARP_SELF_HOSTED=pure: "works" means both lanes give the value.

## Language gaps the source works around (fix the lane, then restore the source)
- `x not in xs` works in Rust's lane, the pure lane says "undefined: in". 20 new sites write `not (x in xs)`
  (emitter.warp:473 480 689 951 956 993 1466 1916, lambdas.warp:219 256 265, node_cases.warp:31, number_keys.warp:83,
  parser.warp:1552, plain_enums.warp:155, prelude_words.warp:65 73 103, uses.warp:36). Restore once the pure lane reads it.
- Top-level names shared by composed stage files replace each other (card two-files-defining). The renames that dodge
  this go back once it is fixed: d0653c507c (emitter: if_else_code, cast_kind, function_head, expression_code, …),
  687e19a224 (length_code), unicode-glyphs 8ca89e6b17 (operator_kind_word), 7e6da82451 (spelled_enums only in
  node_cases.warp).
- `type(n) is key` instead of `n is key` (33 new sites: data_items.warp:67 79 151 152, uses.warp:74 92 101,
  emitter.warp:200 202 205 219 …). This dates from a local `key` that turned a type test into a comparison (200dc0e52c);
  per-function type tests (pure-parser-rest 3a962f49b8) fix that, so `n is key` can come back everywhere.
- emitter.warp:525: an error function's index is stored, not its Function: "a Function read back from the map fails
  with key not found" (card error-index). Restore the Function once that is fixed.
- emitter.warp:760 mentions_name duplicates walk.warp's `mentions`: 8fb61ccf63 says walk.warp's (view_of) failed in
  the emitter. Fix mentions, then drop the copy.
- tries.warp:5 `tries_made = {count: 0}` with `tries_made.count = tries_made.count + 1`: a map so the count can change
  without `global`. lambdas.warp:19 uses `global lambdas_made` plus `+=`. Use one form everywhere (see the fresh-name
  item below).
- `for i, x in xs` fails in the Rust lane ("not a list"); emitter.warp:1195 `for k in 0..<#module.closures` and
  `module.closures[k]#1` would read better with it.
- Known: plain_enums.warp subjects_in reworded around rust-lane-local (a5459772ce); already being restored.

## Duplicates (merge into one shared function)
- **min/max twice**: pure-field-slice adds lib/compiler/extrema.warp, pure-parser-rest adds lib/compiler/min_max.warp,
  both twins of min_max.rs and both by warp-pure-lambdas. Keep one. min_max.warp's `plain_operators` lists Op cases;
  extrema.warp spells them as texts (`(op as text) in plain_operators`).
- Fresh-name counters, five copies: lambdas.warp fresh_number/fresh_name, comprehensions.warp comprehensions_made,
  tries.warp tries_made/try_name, min_max.warp temporary, extrema.warp fresh_extremum/extremum_name. Make it one
  `fresh_name(role)` in walk.warp.
- Node builders, several copies: tries.warp applied_to/braced/chosen_branch, lambdas.warp call_node/block_of/if_block/
  set_to, extrema.warp if_else, min_max.warp folded and plain_enums.warp case_at, which build `if … then … else` by
  hand. Put them in shapes.warp.
- The parts of a node, four copies: scope.warp:132 parts_of, plain_enums.warp:86 children_of, lambdas.warp:175
  walked_parts, plus walk.warp's children (which maps the parts). Make it one `parts_of` in walk.warp.
- A program's statements, two copies: uses.warp:64 statements_in and lambdas.warp:153 is_sequence_of_statements/
  top_statements.
- The names a body assigns, two copies: number_keys.warp:67 names_assigned and lambdas.warp:157 names_bound.
- data_items.warp:85 joined_results is `items.flat_map(f)` (works).
- parser.warp duration scanning (parser-reads-day): lines 1532, 1548 and 1554 copy spaces_end, 1550 copies name_end,
  and parser.warp:46 blank_end_of copies blank_end.
- Scanning to a statement's end, four copies: unicode-glyphs statement_words_at, skip_statement, operator_declaration
  and foreign_forms.warp:78 ruby_fields. Make it one `statement_end(p)`.
- unicode-glyphs: alias_at and user_operator_at are the same longest-match search.
- Dropping a trailing `?` by hand, three copies: emitter.warp:796 and 975 and pure-field-defaults declared_field.
  Make it one `without_optional_mark(name)`.

## Hand-expanded sugar (use the language's own form)
- Index search by filter: plain_enums.warp:131 `numbers.filter(i => cases#i == name)#1` and pure-named-arguments
  placed_arguments `range_list(#names).filter(i => names[i] == name)#1` are `xs.index_of(x)` (works).
- The last item: `xs#(#xs)` is `xs.last` (works) at emitter.warp:1212, analyzer.warp:90 95 477 and
  prelude_words/comprehensions.
- Identity maps `n.map(x => x)` used to turn a group into a list: emitter.warp:243 735 737, analyzer.warp:94. Use
  `items_of(n)`.
- Text from a value: `"" + value` is `"$(value)"` (works in both lanes) at number_keys.warp:94, plain_enums.warp:140,
  parser.warp:1569, emitter.warp:1778 1779, and in unicode-glyphs about ten times (`"" + c`).
- Searches written as loops (unicode-glyphs, parser.warp): `for op in SYMBOL_OPERATORS { if op#1 == target { return
  op#2 } }` three times. SYMBOL_OPERATORS and WORD_OPERATORS as maps would make each a lookup.
  - `in_block = false; for block … { if … { in_block = true } }` is `.any`.
  - blank_words is `split(" ").filter(word => word != "")`.
  - `for mark in opening[1..] { marks = marks + mark }` is `.join("")`.
  - The leveled rebuild in declared_operators is a `map`; library_glyph_operators is a `filter`.
  - declared_level is a search.
  - is_operator_glyph's `while at < glyph.length` with a counter is `for c in glyph`.
- tab_separates (parser.warp:262) uses a counter while to scan `start ..< end` for a tab: `p.code[start ..< end]
  .contains('\t')`.
- comprehensions.warp:41 and 65: a while counting to the first `for` clause. Use a search word, and `+=` for
  `end = end + 1` and `comprehensions_made = comprehensions_made + 1`.
- If-chains on one value where `match` reads better (match on texts works in both lanes):
  - lambdas.warp:125-136 `if word == "map"` …
  - emitter.warp:588-641, the intrinsics `if name == "new_text"` …
  - runtime/types.warp:14-24 `if type == "int"` …
  - analyzer.warp:185-191 `op == "?"`, `"else"`, …
- Operators spelled as texts in stages that otherwise test Op cases (265b78afa3 moved the stages to `is_key(n, Op.X)`):
  - uses.warp:78 94-96 `operator == "="`, `":="`, `":"`.
  - tries.warp:56 `op_of(statements#1) == ":"`.
  - parser.warp:745 `op == "==" or op == "is" or …` is `op in [...]`.
  - extrema.warp plain_operators.
- unicode-glyphs `p.at = p.at + 8` is the length of "operator": write `"operator".length`.
- unicode-glyphs precedence_declaration returns an error as a one-item list and a result as a two-item list. Raise
  the error instead.
- `ALIAS_TARGET_NAMES.has(t) then ALIAS_TARGET_NAMES[t] else ""` is `ALIAS_TARGET_NAMES.get(t) ?? ""`.
- parser.warp:1518 TIME_UNITS lists each unit twice, singular and plural: list the singulars and accept a trailing s.

## Magic numbers
- The runtime tests kinds by number: `kind_code(n) == 8`, `marked_kind(n) == 257`, `kind_code(value) < 5 or … == 10
  or … > 11` (runtime/conversions.warp:62, new in d69680de41). The same in arithmetic, equality, fields and types.
  emitter.warp already names them (node_kind_codes). Give the runtime a Kind enum (plain enums run pure now) and make
  is_null a bool rather than `== 1`.

## Not downgrades (checked, fine)
- plain_enums.warp, lambdas.warp, number_keys.warp and node_cases.warp read well: match on Node cases, Op enums,
  lambdas, `.all`/`.any`/`.map`.
- The runtime's byte-level `while` loops step over UTF-8 widths and growing work lists, which `for` cannot do.
- Comments: no "for now" or "work around" comments were added; the known limits are stated as refusals.

# warp compiles parser.warp: refusal roots and owners

Goal (user 2026-10-19): the self-hosted pipeline compiles lib/compiler/parser.warp. One owner per root cause:
claim a line here (commit straight to main) BEFORE you start, mark it done with the commit, add new roots you find.
Survey: WARP_EMIT_SURVEY over parser.warp with its use'd modules (warp-emitter-structures, 2026-10-19 evening:
28 functions refused, down from 124).

## Done
- field write on a parameter (`p.at = p.at + 1`): emit-enum-match d717c74deb, also pure-lane 4696852d1
- `def f(x): T := e` registers as a definition: emit-enum-match d717c74deb, runtime-declared ae7663fc80
- `def f(x): list|map|any := e` result types in the Rust parser: emit-enum-match d717c74deb
- bracket_of, separator_of, bracketed, separated as runtime words, n.op / n.bracket fields: emit-enum-match d717c74deb
- node into a variable declared list/int: runtime-declared 3a700ae
- a runtime.warp word's declared result kept (`f(): int := { … return xs#i … }` was a node): runtime.warp reaches
  emitter.warp through the analysis view, runtime-declared 305298c5e7
- match with node patterns: pure-lane 54dabdd10 (node_matches.warp)
- bare `return` (`{ return ø }`) and `continue` in while loops: pure-lane ec8cd7ade (a for whose body continues stays
  with Rust: its step must not be skipped)

- `Parser{code: c, at: 0}` of a declared type read as the key `Parser: {…}` (p.branch became a call of branch):
  pure-front d8bc25e098 (warp-parsing; lands with the pure-front merge)
- `x === ø`, `x !== ø`: pure-lane (warp-self-analyzer); `x in xs` → collection_contains: pure-lane
- class constructor `Point(2, 3)`: pure-lane 046b2947d (warp-self-analyzer)

- the file's own globals in functions (PRINT, ARGUMENT_POWER, UNIT_WORDS, …): no root of its own, main was refused
  first (normal lane: the def heads, ae7663fc80, data/self_hosting/parser_gap_with_def.txt names none of them; pure
  lane: `symbol("print")`, warp-self-emitter's); globals read fine in functions (pure/28_global_tables.warp)
- match arms of values (`1:`, `"hi":`, `'a':`, `Color.Red:`): match-arms a3499f85ac (warp-parsing; on pure-lane,
  lands after it; enum cases run once enum lowering does)

- `x is int/list/symbol` type tests: pure-lane 02f878456e (type_tests.warp, rung 29)
- int(x), float(x), `x as int` of texts, characters, floats and ratios; positioned(n, line, column) as a $MetaNode
  read by n.line / n.column; `op as Op` in keyed (shapes.warp's keyed shadowed runtime.warp's keyed intrinsic inside
  key_node): emit-enum-match 45bc9a01bf, 12702665ba (survey 25 → 14 with main merged)

- pending kinds in a cycle of functions (continued, header_expression, keyed, superscript_power, typed_default,
  global_declaration: "kind ?"): a value of a call still compiling is a node, its readers read again at the end,
  runtime-declared 5f7ca4f819
- a write by a run-time key (`m#(k+1) = v`, remember() in scope.warp): runtime.warp node_put, runtime-declared b1076666be

## Open
| root | owner |
|---|---|
| lower/upper (case tables), ran_without_error, contains alias | warp-self-emitter (pure-builtins) |
| prelude defs on demand (is_digit), `symbol("print")` in the pure lane | warp-self-emitter |
| "unknown name any/list" in peek, group_parts, spans, numbers, texts, atoms: return annotations of one-liners `def f(…): list := …` (definitions pass), not type tests | warp-self-analyzer |
| enum lowering (`enum Op {…}`; Op.Colon wrong result) | warp-cache: `Op.Colon` gives Colon on main (87062eb624); the stage's 'not an object' once a stage program has Op (a one-entry map named like a class read as an instance): enum-decls 24a46fd732, Ready (card class-named-entry); a used module's Op/Bracket/Separator declaration and classes (Colon unknown, 71 → 44 refusals): warp-list-result-type parser-compiles 3eefd08fcd; `use "lib/compiler/node.warp"` itself: card node-op-field-type, Later |
| try (Rust lowers it to ran_without_error/is_error: needs those builtins) | warp-self-analyzer |
| a node into a typed Parser field (written_tree), `branch` called with a missing argument | warp-emitter-structures |
| an instance inside a map (gaps) | gone on main 57469d5a52: warp-cache's survey has no refusal naming gaps or an instance (68 refusals, data/self_hosting/parser_gap_map_instances.txt) |
| print of a list with its own bracket and separator (`(1 2)`, `[1; 2]`) and of an error, in runtime.warp node_text, on emit-enum-match's bracket_info | warp-runtime |
| user-named error types, `raise SyntaxError(…)` / `except SyntaxError as e` (card error-types) | warp-parsing |
| two used files defining one top-level name: a compile error naming both (card two-used-files) | warp-parsing |
| "a parameter of kind ?" and "a node where a ? is needed" (7 refusals), lines_below/braced_items (gives ?) (data/self_hosting/parser_gap_map_instances.txt) | warp-cache: pending-kinds 1dd929f39c (emitter.warp settles recursive results): all 9 gone; 4 now show their case name (typed_default Assign, superscript_power Sub, conditioned Do, global_declaration Colon) and continued shows `the unknown name continue`; 65 refusals left (data/self_hosting/parser_gap_pending2.txt) |
| a field named `header` read as the host word response_header (`p.header` → "header needs 2 arguments", Rust lane too), key_node (an int where a text is needed), the text of a closure | warp-list-result-type (branch parser-roots) |
| `!` words in parser.warp (parser-reads-2), then the pure-lane pass count | warp-is-tests |

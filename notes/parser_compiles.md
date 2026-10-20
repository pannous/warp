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
- a node (or another kind) into a typed field of an instance (written_tree's `Parser{code: source, …}`), checked at
  run time as Rust's: not an int / list / text / bool / float; a runtime word's `global trap_detail = …` writes the
  global; a node where a char is needed (atom): emit-enum-match 7103a5bac6, 0582b0de33 (survey 13 left; the 3
  `branch` calls wait on pure-front d8bc25e098)

- pending kinds in a cycle of functions (continued, header_expression, keyed, superscript_power, typed_default,
  global_declaration: "kind ?"): a value of a call still compiling is a node, its readers read again at the end,
  runtime-declared 5f7ca4f819
- a write by a run-time key (`m#(k+1) = v`, remember() in scope.warp): runtime.warp node_put, runtime-declared b1076666be
- lower/upper (Unicode case tables), ran_without_error/try, is_error, contains alias, a missing field `try n.line else 0`
  (walk.warp line_of): pure-builtins d1054cbdc2 (Ready)
- `symbol("print")` in the pure lane: runs pure on pure-lane (rung check 2026-10-19 night)
- the pure lane follows `use` (lexer, shapes, words, sugar reach it), `def f(x): T { body }`, module_source:
  pure-follows-use, on main 48edeefefc (warp-self-analyzer)
- `math.sqrt(4)` read as `math . √(4)` by parser.warp (front lane printed t), `x: bool = v`, `x: int? = v` (line_of,
  column_of): pure-lane-use (warp-self-analyzer). Survey over parser.warp on it (main e0827eafaf merged): 3 refusals,
  caught_error, hex_value, code_escape; the run itself ends fine

## Open
| root | owner |
|---|---|
| prelude defs on demand (is_digit); the pure lane has no prelude at all: parser.warp under WARP_SELF_HOSTED=pure-only stops at `group ( is_alphanumeric` (lexer.warp is_name_char; 2026-10-20 on pure-lane-use): uses.warp splices the prelude definitions a program reaches, card prelude-for-pure | warp-self-analyzer (was warp-self-emitter) |
| "unknown name any/list" in peek, group_parts, spans, numbers, texts, atoms: return annotations of one-liners `def f(…): list := …` (definitions pass), not type tests | pure lane done on pure-lane 2c0201b4e9 (`f(x): T := e` as `f(x) := {the result: T = e; the result}`). Rust's parse of `def f(…): T := e` is right on main (d717c74deb; card def-int-list closed: the survey over lexer.warp has no any/list line, 2026-10-20). Over parser.warp the emit lane now fails with "key not found" before surveying (card emitter-fails), and the pure lane does not follow `use` yet (pure refusal of parser.warp: `tree: group ( view_of`) (warp-self-analyzer) |
| enum lowering (`enum Op {…}`; Op.Colon wrong result) | warp-cache: `Op.Colon` gives Colon on main (87062eb624); the stage's 'not an object' once a stage program has Op (a one-entry map named like a class read as an instance): enum-decls 24a46fd732, Ready (card class-named-entry); a used module's Op/Bracket/Separator declaration and classes (Colon unknown, 71 → 44 refusals): warp-list-result-type parser-compiles 3eefd08fcd; `use "lib/compiler/node.warp"` itself: card node-op-field-type, Later; the pure lane: plain_enums.warp (enum declaration → sum type, Color.Red / Color::Red → Red; bare and dotted case arms in node_matches.warp), warp-parsing pure-enums 5d171b7e3, Ready |
| try (Rust lowers it to ran_without_error/is_error: needs those builtins) | `try x else y` done on pure-lane 2c0201b4e9 (lib/compiler/tries.warp, builtins merged from pure-builtins); `try {…} catch e {…}` lowers too but waits on caught_error in emitter.warp (warp-self-emitter) (warp-self-analyzer) |
| an instance inside a map (gaps) | gone on main 57469d5a52: warp-cache's survey has no refusal naming gaps or an instance (68 refusals, data/self_hosting/parser_gap_map_instances.txt) |
| print of a list with its own bracket and separator (`(1 2)`, `[1; 2]`) and of an error, in runtime.warp node_text, on emit-enum-match's bracket_info | lists and keys (`y+z`) done on runtime-declared 10a97fec6e, 52296900ae; an error's print still open (warp-runtime) |
| user-named error types, `raise SyntaxError(…)` / `except SyntaxError as e` (card error-types) | warp-parsing: error-classes babdef0d22, Ready (decision error-classes: `class X: Error`, except tests the class and its subclasses; notes/error_classes.md) |
| two used files defining one top-level name: a compile error naming both (card two-used-files) | warp-parsing: two-used-files 02aa783f73, in the Integrator's batch (3 clashes in lib/compiler renamed) |
| "a parameter of kind ?" and "a node where a ? is needed" (7 refusals), lines_below/braced_items (gives ?) (data/self_hosting/parser_gap_map_instances.txt) | warp-cache: pending-kinds 3998b7a62d, Ready (emitter.warp settles recursive results): all 9 gone; 4 now show their case name (typed_default Assign, superscript_power Sub, conditioned Do, global_declaration Colon) and continued shows `the unknown name continue`; 65 refusals left (data/self_hosting/parser_gap_pending3.txt) |
| the text of a closure (emitter.warp refused any program with closures that shows values): closure-text 2675fb1978, Ready (card emitter-shows); `header` field: done, main 3dd502e178 (card field-named) | warp-list-result-type |
| survey on pending-kinds 3998b7a62d (main merged): 16 refusals left, the case names are gone (data/self_hosting/parser_gap_pending4.txt) | warp-cache |
| continued: "a text where a char is needed" (the `continue` refusal is gone) | warp-cache: cache-roots b401fc5bb (Ready): an unannotated parameter is typed from the literals every call passes (binding_powers("=") a char, ("to") a text: a text) |
| blank_end giving a mixed | warp-cache: cache-roots b401fc5bb (Ready): `while true` without a break gives no kind, the returns decide |
| dedented: `min(written.map(indent_width))` (empty_extremum unknown) | warp-cache: cache-roots b401fc5bb (Ready): empty_extremum(min) ends the run in min_of_an_empty_list |
| plain `raise "text"` (items, enum_with_values, shader_block, atom, sum_type: group ( raise) | warp-cache: cache-roots b401fc5bb (Ready): runtime.warp raised (trap_detail, returned_error), message as Rust's |
| gaps: "a block or map holding an instance" is back on main | warp-cache: cache-roots b401fc5bb (Ready): a { } group with a statement beside the instance is a block |
| `!` words in parser.warp (parser-reads-2), then the pure-lane pass count | warp-is-tests |
| pure-only over the used files (prelude-for-pure 2026-10-20, with pure-lane-use): words.warp runs pure; positions.warp, lexer.warp: the list comprehension of positions.warp line_widths runs pure (pure-comprehensions: lib/compiler/comprehensions.warp); pure-only now stops at `code_points` (the field codepoints of a text) | warp-parsing |
| shapes.warp, sugar.warp: lambdas, `map(xs, x => …)` / `xs.map(f)`: the pure lane refuses every closure (`group ( map`; Rust lambdas.rs → closure_new) | warp-pure-lambdas |
| parser.warp pure-only with the prelude: stack overflow, then "call stack exhausted" and "out of time" | warp-self-analyzer, prelude-for-pure: the worker thread holds the wasm stack (7286515466); analyzer.warp parameter_name recursed forever on the prelude's labelled `replace(t, old, with new)` (preposition_heads.warp drops the label, rung 32); a stage request gets 600 s CPU (parser.warp lowers in ~70 s); `x is symbol` stayed `==` because words.warp has a map key `symbol:` (type tests ignore fields, rung 33). parser.warp now reaches emitter.warp: survey 2026-10-20 94 refusals: "a parameter of kind Node" 23, "group ( Key" (Node constructors) 21, "a variable declared Node" 5, closures `group ( map` 5, "the unknown name raise" 4, comprehension `line` 1 |
| prelude aliases (`values`, `keys`, `is_alnum`: `alias x = y`) not followed in the pure lane (probes/values_keys/class_map.warp) | warp-self-analyzer: done on prelude-for-pure ab51a16622 |
| key_meanings: "a int where a list is needed" (survey 2026-10-20 on cache-roots + emitter-twin-fails: 6 refusals) | warp-cache: done on parser-roots 2e5233dd70 (Ready), `x as codepoint/integer/string`; survey on it: 3 refusals left, all warp-self-emitter's (caught_error, line_of, column_of) |
| hex_value: `group ( collection_position` | warp-cache: done on parser-roots 2e5233dd70 (Ready) (runtime.warp collection_position, xs.index_of(x)) |
| code_escape: "the unknown name codepoint" | warp-cache: done on parser-roots 2e5233dd70 (Ready) (int as codepoint is a char) |
| parsed: `group ( caught_error` | warp-self-emitter: done on pure-type-tests f07ac4e56c, lands with prelude-for-pure (warp-self-analyzer): emitter.warp's throw keeps the error's message, runtime.warp caught_error/error_of make the Error, `e.message` reads it; a one-statement `try { x } catch …` guards x (tries.warp). parsed's shape runs pure (probes/self_hosting/emitter/core/try_catch.warp). Left: a braceless `raise "…"` statement on the pure lane (card pure-lane-statement), wasm-native traps under try (card self-hosted-try-2) |

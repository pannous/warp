# parse word: real Nodes of a source (card parse-word, topic parser-features)

Unblocks step 7 of tree patterns (notes/matching.md "Tree patterns", probes/matching/tree_patterns.warp block 8).

## What exists (probed 2026-10-13)
- Code as data is a wasm Node already: `x = data (n * 1)` gives the key `n*1`, with `x.op` "*", `x.key` n,
  `x.value` 1, `x == data (n * 1)` yes, and `match x { data(n * 1) => "one" }` matches structurally.
- The Node API (self-node-api, src/wasm_emitter/reflection/node_api.rs) reads and builds Nodes: `n.key`, `n.value`, `n.op`,
  `n.name`, `n.bracket`, `type(n)`, `n#i`, `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`.
- A hole is a plain symbol: `data ($a + 0)` has the key `$a`, a symbol. The parser needs no change for holes.
- `parse("n * 1")` of a constant text folds to `(data n*1)` (src/lowering/words/parse_word.rs, step parse-const). A grouped
  `(data …)` reads like a variable holding data: `.op`/`.key`, either side of `==` (library_words.rs may_hold_object,
  equality.rs is_structured_value).
- Backticks are template text (`Hello ${name}`, with a hint toward double quotes). So `` `n * 1` `` is the text
  "n * 1". The design example's backtick code patterns would collide with that.
- Meta (line:column, comments) is dropped when a Node is emitted, so a data value has no positions.

## Shape (proposal, undoable)
1. Quoted code is `data (…)`, which already parses and compares. The tree-pattern examples spell it `data(…)`, not
   backticks: `` `…` `` stays template text, so no new syntax is needed. A hole `$name` binds in a pattern.
2. `parse(text)`:
   - With a constant text, it folds at compile time to the same Node `data (…)` would give, using the Rust parser, so
     `parse("$a + 0")` == `data ($a + 0)`.
   - With a run-time text, it is a host word like `interpret`'s run_block. src/host/native.rs already reads guest
     Nodes (wasm_reader::node_in) and builds them in the program (build_in_program), and web/playground/host.js
     loads warp.wasm for run_block. So it runs the Rust parser itself, with no parser-twin differences. The
     self-parser (lib/compiler/parser.warp) stays a differential twin; notes/self_hosting.md's "needs a host word
     and a GC reader" is out of date, since both exist.
3. Positions: data keeps Meta. Kind::Meta (10) is emitted as a wrapper with `data` = the node and `value` = a map
   {line, column, comment}. The Node API looks through it (`n.op` of a wrapped key is its op), and `n.line`,
   `n.column` and `n.comment` read it. `==` and matching ignore Meta.
4. Holes are a matching feature, owned by the pattern sessions:
   - `$a` in a pattern binds `a` to the subtree there, and a repeated hole must be equal (`f($x, $x)`).
   - The binding sits in switch.rs pattern_tests next to the map patterns.
   - `$a` outside a pattern stays a plain symbol, so `simplify(data (n * 1))` gives `n`.

## Steps (sub-cards once agreed)
- parse-const: `parse("…")` with a constant text folds to the data Node; test: equal to `data (…)`.
- code-holes (pattern sessions): `$a` binds in match arms and `is`; block 8 of tree_patterns.warp passes.
- data-meta: Meta kept in data Nodes, `n.line` / `n.column` / `n.comment`, ignored by `==`.
- parse-runtime (done, branch parse-runtime): `parse(t)` of any other text is the host word `parse_code(t)`
  (src/host.rs). Natively it is the Rust parser (host/native.rs); in the browser, run_block's compiler instance
  answers a `{parse: text}` request (web.rs eval_block_report, host-compiler.js). A parse failure is an Error that
  `try` catches. A method `x.parse(…)` is never rewritten. `parse_word::code_of` never reads a file, so
  `parse("a.warp")` is code.
- Found on the way (card data-quote-lowered): later lowering passes rewrite inside quoted data, so `data (a.warp)`
  has the op `#`, while the run-time parse keeps `.`. Fixed 2026-10-13: library_words (member access to `#`) and
  declaration_forms (a literal range written out as its list) skip quoted data; tests/node/test_quote_as_written.rs
  checks `data (S) == parse(t)` for a run-time t, so a pass that rewrites a quote again fails there.

## Agreed with warp-operator-shall (2026-10-13)
- Operators stay Keys in data. `&`/`|` are decided at parse time (Op::BitAnd/BitOr for operands that may be ints,
  else And/Or). A single `|`'s right side carries pipe-stage meta until pipes.rs decides, so patterns match through
  drop_meta().
- Code patterns match the canonical form, with both sides normalized by the same rules. BitAnd/BitOr never fold onto
  And/Or.
- In a code pattern, `$a | $b` is the written operator, not arm alternatives (or_operands splits Or and BitOr in case
  keys): alternatives are split only outside `data(…)`.

## Agreed with warp-list-pattern (2026-10-13)
- `data(…)` is the quote in pattern position (backticks stay template text).
- `$name` is a hole; a repeated `$x` must be equal (as `$1` reads elsewhere).
- pattern_tests needs only `type(n)` / `is_type n "key"`, `n.op`, `n.key`, `n.value`, `n#i`, `count n`: all exist.
- Named patterns (branch matching, `pattern Name = …`) make `pattern Zero = data($a + 0)` work once holes are in.
- warp-list-pattern takes code-holes after tree-find (step 5).

## data-meta (done 2026-10-13)
- `$Node` is a non-final WASM GC type; `$MetaNode` is its subtype with `line`, `column` (i64) and `comment`
  (ref null $Node). Everything that reads fields 0–2 is unchanged, and `==` ignores the position.
- Quoted code (`data …`, `parse("…")` folded at compile time) is emitted by emit_literal, which wraps every node
  whose Meta layers carry a LineInfo or a comment through the runtime function node_at.
- `n.line`, `n.column`, `n.comment` read through node_part (NODE_PARTS 6–8); a node without them has no such field.
- Positions of `parse("…")` are in the parsed text (line 1 column 1 is its start).
- A comment ending a line documents that line's item (2026-10-13): `a: 1 // c` gives `(x#1).comment` "c"; before,
  it became the leading comment of the next line. Scanner take_trailing_comment, called by the list item loop.
  Open: `a: 1 # c` is still lost, the expression scanner (skip_spaces_and_inline_comments) drops `# …` mid-line.
  Test tests/node/test_trailing_comment.rs.
- Run-time `parse(t)` keeps positions too (2026-10-13): TaskValue::Positioned (from Node::written_at) is built by
  the program's exported node_at; in the browser tree_of adds `at: [line, column, comment]` and host.js buildValue
  calls node_at. A program mentioning parse_code emits node_at and $MetaNode. Test tests/node/test_parse_positions.rs.

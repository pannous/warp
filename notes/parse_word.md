# parse word: real Nodes of a source

Code patterns (notes/matching.md "Tree patterns", step 7) match quoted code and parsed text alike.

## Code as data
- `x = data (n * 1)` is a wasm Node: the key `n*1`, with `x.op` "*", `x.key` n, `x.value` 1; `x == data (n * 1)` is
  yes and `match x { data(n * 1) => "one" }` matches structurally.
- The Node API (src/wasm_emitter/reflection/node_api.rs) reads and builds Nodes: `n.key`, `n.value`, `n.op`, `n.name`,
  `n.bracket`, `type(n)`, `n#i`, `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`.
- `data(…)` is the quote in pattern position; backticks stay template text (`Hello ${name}`, hinted toward double
  quotes). A hole `$name` is a plain symbol in data and binds in a pattern; a repeated `$x` must be equal. Outside a
  pattern `$a` stays a symbol, so `simplify(data (n * 1))` gives `n`.
- Lowering passes leave quoted data as written (library_words' member access, declaration_forms' literal ranges skip
  it): tests/node/test_quote_as_written.rs checks `data (S) == parse(t)` for a run-time t, so a pass that rewrites a
  quote fails there.

## parse(text)
- A constant text folds at compile time to the same Node `data (…)` gives (src/lowering/words/parse_word.rs), so
  `parse("$a + 0")` == `data ($a + 0)`. A grouped `(data …)` reads like a variable holding data: `.op`/`.key`, either
  side of `==` (library_words.rs may_hold_object, equality.rs is_structured_value).
- Any other text is the host word `parse_code(t)` (src/host.rs): natively the Rust parser (host/native.rs); in the
  browser run_block's compiler instance answers a `{parse: text}` request (web.rs eval_block_report,
  host-compiler.js). A parse failure is an Error that `try` catches. A method `x.parse(…)` is never rewritten.
  `parse_word::code_of` never reads a file, so `parse("a.warp")` is code.
- The self-parser (lib/compiler/parser.warp) is a differential twin of the Rust parser.

## Operators in data
- Operators stay Keys. `&`/`|` are decided at parse time (Op::BitAnd/BitOr for operands that may be ints, else
  And/Or). A single `|`'s right side carries pipe-stage meta until pipes.rs decides, so patterns match through
  drop_meta().
- Code patterns match the canonical form, both sides normalized by the same rules; BitAnd/BitOr never fold onto
  And/Or. In a code pattern `$a | $b` is the written operator, not arm alternatives: alternatives are split only
  outside `data(…)`.

## Positions (data-meta)
- `$Node` is a non-final WASM GC type; `$MetaNode` is its subtype with `line`, `column` (i64) and `comment`
  (ref null $Node). Everything that reads fields 0–2 is unchanged, and `==` ignores the position.
- Quoted code (`data …`, `parse("…")` folded at compile time) is emitted by emit_literal, which wraps every node
  whose Meta layers carry a LineInfo or a comment through the runtime function node_at.
- `n.line`, `n.column`, `n.comment` read through node_part (NODE_PARTS 6–8); a node without them has no such field.
  Positions of `parse("…")` are in the parsed text (line 1 column 1 is its start).
- Run-time `parse(t)` keeps positions too: TaskValue::Positioned (from Node::written_at) is built by the program's
  exported node_at; in the browser tree_of adds `at: [line, column, comment]` and host.js buildValue calls node_at.
  A program mentioning parse_code emits node_at and $MetaNode. Test tests/node/test_parse_positions.rs.
- A comment ending a line documents that line's item: `a: 1 // c` gives `(x#1).comment` "c" (scanner
  take_trailing_comment, called by the list item loop; tests/node/test_trailing_comment.rs). Open: `a: 1 # c` is
  lost, the expression scanner (skip_spaces_and_inline_comments) drops `# …` mid-line.

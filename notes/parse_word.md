# parse word: real Nodes of a source (card parse-word, topic parser-features)

Unblocks step 7 of tree patterns (notes/matching.md "Tree patterns", probes/matching/tree_patterns.warp block 8).

## What exists (probed 2026-10-13)
- Code as data is a wasm Node already: `x = data (n * 1)` gives the key `n*1`, with `x.op` "*", `x.key` n,
  `x.value` 1, `x == data (n * 1)` yes, and `match x { data(n * 1) => "one" }` matches structurally.
- The Node API (self-node-api, src/wasm_emitter/node_api.rs) reads and builds Nodes: `n.key`, `n.value`, `n.op`,
  `n.name`, `n.bracket`, `type(n)`, `n#i`, `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`.
- A hole is a plain symbol: `data ($a + 0)` has the key `$a`, a symbol. The parser needs no change for holes.
- No `parse` word: `parse("n * 1")` is "undefined function: parse".
- Backticks are template text (`Hello ${name}`, with a hint toward double quotes). So `` `n * 1` `` is the text
  "n * 1". The design example's backtick code patterns would collide with that.
- Meta (line:column, comments) is dropped when a Node is emitted, so a data value has no positions.

## Shape (proposal, undoable)
1. Quoted code is `data (…)`, which already parses and compares. The tree-pattern examples spell it `data(…)`, not
   backticks: `` `…` `` stays template text, so no new syntax is needed. A hole `$name` binds in a pattern.
2. `parse(text)`:
   - With a constant text, it folds at compile time to the same Node `data (…)` would give, using the Rust parser, so
     `parse("$a + 0")` == `data ($a + 0)`.
   - With a run-time text, it calls lib/compiler/parser.warp's `tree` (self-parser), turned into a Node with the
     builders. It needs no host word and runs in the browser too. Its differences from the Rust parser are those of
     the parser twin, which tests/parser/test_self_differential.rs already tracks.
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
- parse-runtime: a run-time text goes through parser.warp `tree`; test: the parse equals `data (…)` for the
  samples the twin covers.

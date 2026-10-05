# Blocks (wiki/charged.md, released 2026-10-05): package 1

## Stage 1 (branch blocks-1): src/lowering/blocks.rs, a source pass before mutation.rs
- `x : e` as a statement with a computed e (an operation or a call) binds x to the uncharged block e; a got-it warning
  where it is written (topic `uncharged-block`: "x keeps the block a+b, it runs only at x!; write x = a+b for its value").
- Literals and words after `:` are that value/symbol (`x : 3`), types and objects keep their meaning (`x : int[100]`,
  `x : 100 * int`, `person: {…}`); a branch `c : x`, a switch case `1: 10` or an object entry is no statement.
- `x!` / `x!!` (parser: `!!` is marked fully, mutation::marked_fully) inline the constant block where they are written,
  so its names resolve at the `!` (`y = 3; w : y*y; y = 4; w!` → 16).
- A bare `x` is the block as data (`data e`); `x` in arithmetic or a comparison is the type error "x is a block (1+2),
  no value: run it with x!, or write x = 1+2 for its value". `x = …` ends the block.
- `data e` is e as written, nothing evaluated: WasmGcEmitter::emit_literal; every operator now has a code in a Key's kind
  (operators.rs OP_CODES, codes 0-4 unchanged), so `data 1+2` reads back as 1+2.

## Stage 2 (branch blocks-2)
- `x = {1+2}`, `x = {a = 1; a*2}`: statements in braces (several, or one computed expression) are a block, silently;
  `{1 2}` stays a data list, `{"A": 1}` a map, `{it*2}` / `{x => x+1}` lambdas.
- `o = {s1: a+b, s3 = a+b}`: a computed `:` entry is an uncharged block the object holds as data (got-it warning where
  written), `o.s1!` runs it where written (the parser marks the field as for `x.upper!`), `o.s1 + 1` is the type error;
  `s3 = a+b` is a value entry (evaluated now); `:=` entries are untouched (P71). An object without such entries is not
  rebuilt (meta attributes like `@unit("cm") {x:1}` stay).
- `help = {print: "there is help"}; help!` runs the object as code: each `key: value` is the call `key(value)`.
Next: code/block prefixes, block parameters for if/while/for/def.

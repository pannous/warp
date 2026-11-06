# Patterns as values: `find` as a plain warp function (card pattern-parameters)

Question: can `tree find {type: "image", src}` be a pure warp function,
the pattern passed unevaluated like a `block`/`data` argument ("pattern as a subtype of data")?

## How it works today
- `page find {type: "image", src}` is infix sugar (extensions/sugar/arms.warp) that rewrites to
  `find_matches(page, data(pattern))`. The runtime matcher lives in lib/prelude.warp
  (`pattern_bindings`, `fits_pattern`, `find_matches`, `replace_matches`).
- Call forms (card pattern-parameters, in lib/text.warp): a parameter typed `data` keeps the
  pattern unevaluated (decision data-parameter), so bare names bind:
  ```warp
  find(page, {type: "image", src})            # ["logo.png", "cat.jpg"]
  replace(page, {type: "image", src}, {type: "link", href: src})
  p = data {type: "image", src}
  find_matches(page, p)                       # pattern held in a variable
  ```
  `type pattern = data` does **not** keep the quoting (aliases lose `Takes` behaviour); write
  `p: data` for a pattern parameter. A dedicated `pattern` parameter type (route A) would need core.

## What the Rust implementation already had (historical)
- Unevaluated arguments: `f(b: block) := b; f({type: "image", src})` → `{type:"image", src}`; `data {…}` likewise.
- Call-site expansion: a function with a `block` parameter is expanded at every call.
- Literal and named patterns (`pattern P = {…}`) match anywhere, also inside functions.

## What was missing (route B, runtime)
1. ~~No `data`/`pattern` parameter type~~ — `p: data` works (decision data-parameter).
2. ~~No matching against a pattern held in a variable~~ — `pattern_bindings` / `find_matches` take a
   `data {…}` value at run time.
3. A recursive block function can hang the compiler (expansion; needs specialization) — still open.
4. RESULT ("the names this pattern bound") — `pattern_bindings` returns that map, or ø.

## Two routes
- A, compile time: a `Takes::Pattern` parameter, monomorphized per distinct pattern (needs core).
- B, run time (done in lib): `pattern_bindings` / `find_matches` / `replace_matches`, plus
  `find`/`replace` call forms with `data` parameters in lib/text.warp. Literal patterns can later
  be specialized as in A as an optimization.

## What it saved
tree_walks.rs walker templates became lib warp (`find_matches` / `replace_matches`). Named-pattern
templates and some `data(…)` replace cases still have gaps (card tree-find-replace-gaps).

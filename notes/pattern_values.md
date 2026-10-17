# Patterns as values: `find` as a plain warp function (card pattern-parameters)

Question (user, 2026-10-17, parser-rewrite context): can `tree find {type: "image", src}` be a pure warp function,
the pattern passed unevaluated like a `block`/`data` argument ("pattern as a subtype of data")?

## How it works today
- `page find {type: "image", src}` has no dot because `find` is an infix word: lowering/collections/tree_walks.rs
  turns `subject find pattern` into a call of a recursive walker generated per use (FINDER template), whose
  `match NODE { PATTERN => FOUND += [RESULT] }` tries the pattern at every depth.
- `src` needs no definition: in a pattern a bare name binds (`{src}` = key src, value bound to `src`), the same
  `pattern_bindings` as `match` (lowering/control/switch.rs). RESULT: one name → its values, several → a map, none →
  the matching subtrees.
- `page.find({…})` and `find(page, {…})` are NOT the tree walk: they fall to `find(list, function)`
  ("find needs a function").

## What the Rust implementation already has (probed with /Users/me/dev/bin/warp)
- Unevaluated arguments: `f(b: block) := b; f({type: "image", src})` → `{type:"image", src}`; `data {…}` likewise.
- Call-site expansion: a function with a `block` parameter is expanded at every call (lowering/control/blocks.rs,
  `Takes::{Value, Block, Callable}`), the hook a pattern parameter would need.
- Literal and named patterns (`pattern P = {…}`) match anywhere, also inside functions.

## What is missing
1. No `data`/`pattern` parameter type: `f(p: data) := …` → "unknown type data of parameter p".
2. No matching against a pattern held in a variable, and it fails SILENTLY:
   `p = data {…}; match m { p => 1; _ => 0 }` → 0, `m is p` → no; in a block function `match n { p! => … }` → 0,
   `n is p!` → no. Patterns exist only at compile time.
3. A recursive block function hangs the compiler: `walk(n: int, body: block) := { if n > 0 { body!; walk(n - 1, body) } }`
   never finishes (expansion inlines into itself forever; needs one specialization per argument instead).
4. RESULT ("the names this pattern bound") has no warp-level word.

## Two routes
- A, compile time: a `Takes::Pattern` parameter, monomorphized per distinct pattern (fixes 1, 3) plus a bindings
  word (4). Fast, still pattern-as-syntax.
- B, run time (recommended for the self-hosted parser): a primitive `node matches pattern` → map of bindings or
  nothing (fixes 1, 2). Then
  ```warp
  find(tree, p: pattern) := {
    found = []
    if b = tree matches p { found += [b] }
    for child in children(tree) { found += find(child, p) }
    found
  }
  ```
  Patterns become first-class data (built, passed, stored). A literal pattern can later be specialized as in A as an
  optimization.

## What it would save
tree_walks.rs (~200 lines: walker templates, placeholder renaming, infix detection) becomes a few lines of lib warp
for `find` and `replace`. switch.rs (~825) stays as the compiled fast path of literal `match`, unless the runtime
matcher replaces it too; named_patterns.rs (~220) would shrink to `pattern P = …` being an ordinary data constant.

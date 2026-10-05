# Function broadcasting (wiki/broadcasting.md, wiki_features row 24)

`src/lowering/broadcasting.rs`, a meaning pass right before `lambdas::lower`. Tests: tests/functions/test_broadcasting.rs.

- A function of one undeclared parameter that its body uses as an arithmetic operand (`square:=it*it`, `square(x):=x*x`)
  broadcasts when called with a list literal: `square [1 2 3]` → `[square(1) square(2) square(3)]`, unrolled at compile
  time. Pairs keep their keys: `square [a:1 b:2]` → `[a:square(1) b:square(2)]`. Nested lists broadcast again
  (`square [[1 2] [3]]` → `[[1 4] [9]]`).
- A variable only ever assigned a non-object list literal is mapped: `square xs` → `map(xs, square)`.
- A function that uses its parameter as a list (`count(xs)`, `xs#2`) takes the list whole: no broadcast.
- Operators never broadcast (`[1 2 3]*2` asks repeat or multiply each element, Footguns "Lists and arithmetic").
- Lawful by construction (notes/laws.md "lawful lifting"): the rewrite is element-wise application of a pure function,
  so `map id = id` and `map (f∘g) = map f ∘ map g` hold; no per-function proof needed.

- P50 (user, 2026-10-05): a parameter declared with a scalar type (`x:int`, `t:text`, the wiki's `square number = …`)
  broadcasts too; `xs:list` takes the list. `print [1 2 3]` keeps printing the list.

Open: lists only known at run time (a parameter, a function result) are not broadcast;
multi-argument folding (`sum [1 2 3]` as `sum(1, sum(2, 3))`) is not done.

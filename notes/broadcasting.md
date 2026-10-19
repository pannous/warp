# Function broadcasting (wiki/broadcasting.md, wiki_features row 24)

`src/lowering/collections/broadcasting.rs`, a meaning pass right before `lambdas::lower`. Tests: tests/functions/test_broadcasting.rs.

- A function of one undeclared parameter that its body uses as an arithmetic operand (`square:=it*it`, `square(x):=x*x`)
  broadcasts when called with a list literal: `square [1 2 3]` → `[square(1) square(2) square(3)]`, unrolled at compile
  time. Pairs keep their keys: `square [a:1 b:2]` → `[a:square(1) b:square(2)]`. Nested lists broadcast again
  (`square [[1 2] [3]]` → `[[1 4] [9]]`).
- A variable only ever assigned a non-object list literal is mapped: `square xs` → `map(xs, square)`.
- A function that uses its parameter as a list (`count(xs)`, `xs#2`) takes the list whole: no broadcast.
- Operators never broadcast (`[1 2 3]*2` asks repeat or multiply each element, Footguns "Lists and arithmetic").
- Lawful by construction (notes/laws.md "lawful lifting"): the rewrite is element-wise application of a pure function,
  so `map id = id` and `map (f∘g) = map f ∘ map g` hold; no per-function proof needed.

- P50: a parameter declared with a scalar type (`x:int`, `t:text`, the wiki's `square number = …`)
  broadcasts too; `xs:list` takes the list. `print [1 2 3]` keeps printing the list.

Open: lists only known at run time (a parameter, a function result) are not broadcast;
multi-argument folding (`sum [1 2 3]` as `sum(1, sum(2, 3))`) is not done.

## Vector parameters (card vector-params-cpu, min-max)
A shader's helpers take vectors and numbers alike (`fract(p)`, `max(c, 0)`); the CPU runs the same functions.
src/lowering/collections/vector_parameters.rs finds the untyped parameters a list may reach: passed a list literal, a
range, a list variable or a call of a function whose result is a list (list_functions), also through another such
parameter, or used as a vector in the body (element-wise operand, `p#2`, `p.xy`). In broadcasting.rs a scalar word of
such a value is `each_any`: `if p is list then map(p, …) else f(p)`, decided at run time; min/max of two values that
may be lists are element_wise_any of the extremum (vector_extremum); lib/math.warp clamp is min(max(…)).
Related fixes: a function reading components of its parameter or of a variable computed element-wise from it never
broadcasts (reads_components); an element-wise value is a list variable only when a side is known to be a list; the
analyzer holds a parameter passed lists and numbers as a Node, an item of a run-time value (Kind::Data) as a Node, and
negates a Node by node arithmetic. Check: probes/ocean/vector_params.warp, vector_min_max.warp; ocean(x, y) of
samples/gpu/ocean.warp on the CPU gives the shader's pixels.

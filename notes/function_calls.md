# Function calls (functions worker, 2026-10-06)

Ported call forms from Python, Kotlin, Swift, C#, JS/TS, Ruby and Julia; tests in tests/functions/, the quick case
list in probes/function_calls.md (run probes/function_calls.sh after scripts/own-warp.sh).

## Decided forms (defaults taken, Interviewer asked where marked)
- Defaults: `f(a, b=2)`, typed `f(a, b:int=2)`, a default may read earlier parameters `f(a, b=a*2)`. Named
  arguments `f(b=1)` / `f(b:1)` skip defaults and mix with positional ones (P37).
- Overloading by arity: `def f(a)` + `def f(a,b)` become `f·1`, `f·2` (src/lowering/overloads.rs
  `lower_arity_overloads`, a MEANING pass before broadcasting reads arities). A call names the variant its argument
  count fits; none or two fitting (`f(a)` + `f(a, b=2)` called `f(5)`) is an error. Methods: class_methods prepends
  `self`, so a method's variant number is one more than written.
- Rest parameters: `xs...` (Julia/Swift), `...xs` (JS), `*xs` (Python) all mean the same (the parser turns `...xs`
  into the starred `*xs` of tuples.rs). Only the last parameter. `total(1,2,3)` → `total([1 2 3])`.
- Spreading: `f(...xs)`, `f(xs...)`, `f(*xs)`: into a rest parameter the list is passed as is (`[2] + xs` after other
  leftovers), into fixed parameters as `xs#1, xs#2, …`. Static: no run-time arity check, a plain call
  (src/lowering/variadic.rs, a SOURCE pass before named_arguments).
- Prefix nesting: `square square 2` / `square(square 2)` → `square(square(2))` when the call has more juxtaposed
  arguments than its function takes (broadcasting.rs `lower_prefix_calls`, before the function value passes).
- Ruby keyword parameters `def f(a, b: 2)`: a literal after the colon is the default (named_arguments.rs).
- Result types before a block body: `-> int { … }`, `: number { … }`, `-> int: body` (declarations.rs `typed_result`).
- A function body block of one braceless call `def f(x){square x}` is that call (library_words.rs `body_statement`).
- Partial application stays `add(1, _)`; `add(1)` is a missing-argument error.

- Closures changing an enclosing variable (P124 asked, default A forced by test_partial_application's counter): a
  lambda shares it (`()=>{ n+=1; n }` counts 1, 2, …), `nonlocal n` may be written; a nested def needs `nonlocal n`,
  else a loud error. nonlocal_cells.rs `lower_lambdas` hoists such a lambda into a nested def `lambda·N` + reference.

- Optional parameters `x?`, `x: int?` mean `x=ø` (the type is dropped: the value is held boxed); `a ?? b` (Op::Coalesce,
  right-assoc, just above `or`) is `if a == ø then b else a`, a computed once (library_words.rs `lower_coalesce`).
  `maybe x`, `maybe int x`, `x: maybe int` are the same (user, P125: "x=ø, maybe x, or x? Same as with optional
  types"). `??` itself was not addressed: default stands. P124 decided A (lambdas share).
- Anonymous functions `function(a, b) {…}`, `fn(x) {…}`, `lambda x: …` (welcome_forms.rs); C `void f() {…}`.
- Recursion over slices `xs#1 + s(xs[1:])`: a branch of Int and run-time number is Data (inference.rs branches_kind).

## Open
- Board cards `functions-*` (todo list): slices in recursion, lambda spellings (`fn`, `lambda x:`, `{|x|}`), C-style
  and Swift-label definitions, Ruby `add 1, 2`, returning a block with `it`, bare nested function names (P82).

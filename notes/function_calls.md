# Function calls (functions worker, 2026-10-06)

Ported call forms from Python, Kotlin, Swift, C#, JS/TS, Ruby and Julia; tests in tests/functions/, the quick case
list in probes/function_calls*.md (run probes/function_calls.sh <file> after scripts/own-warp.sh). Every case that
passes goes into tests/functions/test_ported_calls.rs (one table), so no ported form breaks unseen.

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
- Swift labels `func greet(person name: String)`: the parameter is `person`, the body starts `name = person`; `_ x: Int`
  is `x: Int` (declarations.rs `labeled_parameter`; partial_application skips `_` before a typed parameter).
- Named arguments reach functions with a result type (named_arguments.rs `untyped_head`, `with_parameters`).
- A function returning a character boxes it as a codepoint (user_function_calls.rs), not new_int.
- Recursion over slices `xs#1 + s(xs[1:])`: a branch of Int and run-time number is Data (inference.rs branches_kind).
- Sorting with a function (lambdas.rs, the `sort` iteration): `sort/sorted/sort_by/sortBy/sortedBy`, a comparator of
  two items (a comparison body is "first before second", Swift `by: >`; any other result sorts first when negative, JS
  `(a,b) => a-b`) or a key of one item (Python `key: s => s.length`, Kotlin `sortedBy { … }`). Labels `by:`/`key:`.
  A stable insertion sort of a copy, inlined like map. `sorted(xs)` is `sort(xs)`.
- Lambda parameters between bars, Rust `|a, b| a*b`, Ruby `{ |x| x*x }` (parser expressions.rs `pipe_parameters`,
  only at the start of an expression; wiki closure.md). A block of one arrow lambda is that lambda.
- Ruby `add 1, 2` / `x = add 1, 2`: a call short of arguments takes the items after its comma (broadcasting.rs
  `with_comma_arguments`).
- A block with `it` given as a value inside a function of one parameter (returned, assigned, an argument) is a
  function of its own `it`: `mk(k) := { return {it * k} }` returns `it => it * k` (declarations.rs `bind_it`);
  elsewhere in the body `it` is the parameter (`f(x) := x + it`).
- P82 educate: `def make(){ def inc(){ 1 }; inc }; c = make(); c()` is a compile error naming `function inc`
  (function_values.rs `refuse_called_bare_results`); `make()` alone stays the call of inc.
- Swift closures `{ x in x*2 }`, `{ a, b in a+b }` (the body must use a name, else `{ x in xs }` stays membership) and
  shorthand arguments `{ $0 + $1 }` (lambdas.rs `swift_closure`, `shorthand_parameters`; blocks.rs leaves them lambdas).
- Output words take a prefix call: `puts add 1, 2` → `puts(add(1, 2))` (broadcasting.rs, OUTPUT_WORDS arity 1).
- C#: `int b = 2` is `b:int = 2` (parser typed_parameter), `static int Add(int a, int b) => a + b` defines Add
  (declarations.rs c_function).
- Rest parameters: Kotlin `vararg xs: Int`, C# `params int[] xs` → `*xs` (parser REST_MARKERS); variadic.rs finds
  heads under a result type too.
- Julia's dot call `f.(xs)`, `add.(xs, 10)`: `map(xs, broadcast_item => f(broadcast_item, 10))` (parser
  SpecialInfix::DotCall, only a name followed by `.(`).
- Iteration templates append with `out = out + [x]`: `out.add(x)` was taken by a user function `add`.
- Python `list(x)` is `x as list` (library_words EXPANDED_WORDS); a tuple result type `-> (Int, Int) {…}` is dropped,
  the body's tuple stays (declarations.rs `tuple_result`).
- Python `list()` is `[]` (library_words `empty_list_calls`, unless the program defines list).
- Spreading into a function parameter `def apply(f, *args){ f(*args) }`: variadic.rs keeps the spread of a call of a
  parameter, function_values.rs runs variadic::lower again once f is known.
- Python colon definitions with parameters become definitions in lower_c_functions (keyword_definition), so the
  function value passes see them; without parameters they stay def forms for late_binding (a `z() := e` is a getter).

## Call efficiency (probes/call_benchmark.sh [N], 10^8 calls each)
- Plain, default, named, overload and lambda calls compile to the same direct `call $f` with i64 arguments: equal
  within noise (the machine is shared, runs vary up to 2x).
- A declared result type `-> int` lowered to `body as int`, which boxed (new_int) and unboxed (get_int_value) on every
  call: 5x a plain call. Fixed: `x as int` of an Int emits the i64 (values.rs, casts.rs `emit_int_value_truncated`,
  test_call_efficiency). The ratio truncation (`exact_trunc` when the range may leave fixnums) stays.
- A returned closure `h = mk(1); h(i)` was 4-6x a plain call (closure_call_1 tests and casts the $Closure twice, calls
  through call_ref, the entry unboxes each capture per call). Now even with a plain call:
  - a closure variable whose targets (closure_variable_targets) are one function, and no parameter of that name
    anywhere, calls that target's entry directly (wasm_emitter/closures.rs `emit_direct_closure_call`);
  - assigned once with captured values, lowering/closures.rs `hoist_captures` reads them once after the assignment,
    `h·capture·0 = closure_lambda_1·captured·0(h)` (a capture reader, its kind the target's parameter kind via
    closure_call_kind), and the call passes the locals to the target itself (`emit_hoisted_closure_call`). Nested
    functions with capture globals keep the entry, which restores them.

## Open
- Board cards: functions-csharp (mine); functions-julia (warp-66); functions-sort-op, functions-key, functions-foreach,
  functions-lambda-defaults (warp-14); functions-python, functions-ruby-def (warp-66). Cases: probes/function_calls_more.md.

## Python and Ruby definitions (cards functions-python, functions-ruby-def)
- `def f(*args): body` (colon body, Python): declarations::keyword_definition reads `def head: body` as `head := body`
  early, so variadic.rs sees the rest parameter (it ran before the later pass that knew the colon form).
- `def f(x, **kw): …` (Python): the parser reads `**kw` as the symbol `**kw` (atoms.rs); variadic.rs passes the named
  arguments no fixed parameter takes as one object, `f(5, a=1)` → `f(5, {a: 1})` (`{}` when none). `f(**m)` spreads an object: into `**kw` it is
  the object itself (a fixed parameter named like one of its fields does not take it out; `f(**m, k=v)` is an error
  for now), into fixed parameters the fields for the parameters the other arguments leave (`g(1, **m)` →
  `g(1, b=m.b, c=m.c)`); into a function not defined in the program it is an error.
- Julia `function f(x; y=2) … end`, `f(3; y=4)` already read as `f(x, y=2)` / `f(3, y=4)`; the `end` body is
  end_definitions'.
- Ruby `def f(a, b: 2) a * b end` and `def f(a)` ⏎ statements ⏎ `end`, also `def h` without parentheses:
  declarations::end_definitions (start of lower_c_functions) gives them a `{…}` body. Nested `if … then … end` keeps
  its own `end` (the parser takes it).

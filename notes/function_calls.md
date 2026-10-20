# Function calls

Ported call forms from Python, Kotlin, Swift, C#, JS/TS, Ruby and Julia; tests in tests/functions/, the quick case
list in probes/function_calls*.md (run probes/function_calls.sh <file> after scripts/own-warp.sh). Every case that
passes goes into tests/functions/test_ported_calls.rs (one table), so no ported form breaks unseen.

## Forms
- Defaults: `f(a, b=2)`, typed `f(a, b:int=2)`, a default may read earlier parameters `f(a, b=a*2)`. Named
  arguments `f(b=1)` / `f(b:1)` skip defaults and mix with positional ones (P37).
  A bare flag name sets it: `f(3, loud)` of a parameter `loud = no` (default yes or no) is `f(3, loud: yes)` when no
  variable loud is in scope, as `x.copy(shallow)` is `x.copy(shallow: yes)` (named_arguments.rs
  flag_arguments).
- Overloading by arity: `def f(a)` + `def f(a,b)` become `f·1`, `f·2` (src/lowering/definitions/overloads.rs
  `lower_arity_overloads`, a MEANING pass before broadcasting reads arities). A call names the variant its argument
  count fits; none or two fitting (`f(a)` + `f(a, b=2)` called `f(5)`) is an error. Methods: class_methods prepends
  `self`, so a method's variant number is one more than written.
- Rest parameters: `xs...` (Julia/Swift), `...xs` (JS), `*xs` (Python) all mean the same (the parser turns `...xs`
  into the starred `*xs` of tuples.rs). Only the last parameter. `total(1,2,3)` → `total([1 2 3])`.
- Spreading: `f(...xs)`, `f(xs...)`, `f(*xs)`: into a rest parameter the list is passed as is (`[2] + xs` after other
  leftovers), into fixed parameters as `xs#1, xs#2, …`. Static: no run-time arity check, a plain call
  (src/lowering/definitions/variadic.rs, a SOURCE pass before named_arguments).
- Prefix nesting: `square square 2` / `square(square 2)` → `square(square(2))` when the call has more juxtaposed
  arguments than its function takes (broadcasting.rs `lower_prefix_calls`, before the function value passes).
- Ruby keyword parameters `def f(a, b: 2)`: a literal after the colon is the default (named_arguments.rs).
- Prepositions as argument labels (Swift's labels): a parameter written
  after a preposition, `play(tone, for duration)`, `move(x, from a, to b)`, is passed by that word, which in a call
  always takes a colon: `play 440Hz for: 0.5s`, `play(440Hz, for: 0.5s)`,
  `play tone:440Hz for:0.5s`, `move 1 from: 10 to: 3` (any order), `r = move 1 to: 3 from: 10`, `s + f 9 to: 4`
  (the parser reads `word: ` after a call's argument as its label, suffixes.rs labelled_argument). A call without
  brackets also names arguments: `f b=1 a=5`. Words: to of from with in into at by for
  on each (type_name_matching.rs PREPOSITIONS; preposition_calls.rs is a FUNCTION pass before named_arguments, which
  picks the overload the names fit). Without the colon a word keeps its meaning, `for` loops, `1 to 5` a range, `x in
  xs` a membership; in a call of a function taking it, the bare word (`move x to 5`) is an error naming the colon
  form, or a warning where it cannot mean anything else (`play C4 for 1s`, `melody xs each 1 beat`: for and each,
  preposition_calls.rs UNAMBIGUOUS_LABELS). A variable named `by` stays a variable; a label the function does not take is an error naming those it takes.
  `as` labels a parameter too: `save(made, as file)` is called `save sound as: "x.wav"`
  (type_name_matching.rs AS_LABEL).
  Standard-library functions use the same form: `"I like tea".replace "tea" with: "coffee"`.
- Result types before a block body: `-> int { … }`, `: number { … }`, `-> int: body` (declarations.rs `typed_result`).
- A function body block of one braceless call `def f(x){square x}` is that call (library_words.rs `body_statement`).
- Partial application stays `add(1, _)`; `add(1)` is a missing-argument error.

- Closures changing an enclosing variable (P124): a
  lambda shares it (`()=>{ n+=1; n }` counts 1, 2, …), `nonlocal n` may be written; a nested def needs `nonlocal n`,
  else a loud error. nonlocal_cells.rs `lower_lambdas` hoists such a lambda into a nested def `lambda·N` + reference.

- Optional parameters `x?`, `x: int?` mean `x=ø` (the type is dropped: the value is held boxed); `a ?? b` (Op::Coalesce,
  right-assoc, just above `or`) is `if a == ø then b else a`, a computed once (library_words.rs `lower_coalesce`).
  `maybe x`, `maybe int x`, `x: maybe int` are the same (P125).
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
- Getters and actions: `speak := …` and `speak() := …` both run the body at every use;
  `:=` is the canonical form (examples, kitchensink: `speak := name + " barks"`, read as `rex.speak`). The empty
  parentheses stay for actions: a body with effects (`step() := { cells = next }`, `print`, `random()`) written
  `step := …` warns "step runs … at every read", since a bare read should not change anything; `step()` says "call".
  A constant body (`x := "hi"`) hints `x = "hi"`.
- Python colon definitions with parameters become definitions in lower_c_functions (keyword_definition), so the
  function value passes see them; without parameters they stay def forms for late_binding (a `z() := e` is a getter).
- OCaml/F# `let f x = body in rest` (welcome_forms.rs `let_binding`); JS IIFE `(() => 7)()` (parser: a glued `()`
  after a one-item group is a call with no arguments); Go `(int, int)` results convert each returned value
  (declarations.rs `typed_returns`) and `q, r := f()` assigns (welcome_forms.rs `go_destructuring`); `reduce` with a
  start value is `fold` (lambdas.rs `with_start`).
- Python generator argument `sum(x*x for x in xs)` (comprehensions.rs), `any(xs)`/`all(xs)` test the items;
  Swift labels on function-typed parameters (body renamed to the label; a lambda block is a named argument).
- P142: prefer broadcasting `square [1 2 3]` and `square all xs` (wiki/all.md) over map in tests, hints, docs.
  `f all xs` → map with a lambda (broadcasting.rs `all_calls`); library words of one value (`upper`, `round` …) and
  `abs`/`sqrt`/`cbrt` broadcast over a list literal or list variable. P143: `f x == y` is `(f x) == y` when f's
  first parameter is typed as no truth value, else a loud error naming both forms. probes/broadcasting_forms.md.
- Broadcasting (P142) covers: lambda variables, library words over a call (`sum square xs`), several parameters
  (`add [1, 2] 10`, the one list argument to a scalar parameter is mapped), `all` in any argument (`add(all xs, 10)`),
  nested literals (`square all [[1, 2], [3]]` spliced at compile time), lists built by appending; `max xs` without
  parentheses (min_max.rs). Same speed as map (call_benchmark.sh list rows): all lower to the same loop.

## Call efficiency (probes/call_benchmark.sh [N], 10^8 calls each)
- Plain, default, named, overload and lambda calls compile to the same direct `call $f` with i64 arguments: equal
  within noise (the machine is shared, runs vary up to 2x).
- A declared result type `-> int` costs nothing: `x as int` of an Int emits the i64 (values.rs, casts.rs
  `emit_int_value_truncated`, test_call_efficiency). The ratio truncation (`exact_trunc` when the range may leave fixnums) stays.
- A returned closure `h = mk(1); h(i)` is as fast as a plain call (the generic path, closure_call_1 through call_ref
  with captures unboxed per call, is 4-6x slower):
  - a closure variable whose targets (closure_variable_targets) are one function, and no parameter of that name
    anywhere, calls that target's entry directly (wasm_emitter/closures.rs `emit_direct_closure_call`);
  - assigned once with captured values, lowering/definitions/closures.rs `hoist_captures` reads them once after the assignment,
    `h·capture·0 = closure_lambda_1·captured·0(h)` (a capture reader, its kind the target's parameter kind via
    closure_call_kind), and the call passes the locals to the target itself (`emit_hoisted_closure_call`). Nested
    functions with capture globals keep the entry, which restores them.

## More ported forms (probes/function_calls_round5.md … round7.md)
- Haskell `twice x = x * 2; twice 4` among statements (declarations.rs `haskell_definitions`: a later statement uses
  the name, the body every parameter; type words, library words, `print x = 5` stay); R's `f <- function(x) x * 2`
  and braceless `f = function(x) x * 2` (welcome_forms.rs `assigned_braceless_function`); JS destructured parameters
  `({a, b}, k) => …` (welcome_forms.rs `destructured_parameters`, `object·0.a` fields).
- P145: `x <- 3` assigns with a got-it note "write x = 3", the cramped `x<-3` (and `x<- 3`) is an error naming
  `x = 3` and `x < -3`, `x < -3` and `x <-3` compare (scanning.rs `left_arrow_assignment`). `<-` stays reserved: no
  docs or new tests beyond P145's.
- Elixir `fn a, b -> body end` (parser atoms.rs `parse_elixir_function`, needs `->` on the line and a closing `end`;
  Rust's `fn(x) -> i32 { … }` stays); a dot call on a number `f.(4)` is the call. Typed Kotlin lambdas
  `{ a: Int, b: Int -> a + b }` and Swift closures `{ (x: Int) -> Int in x * x }` (lambdas.rs `arrow_parts`); Ruby
  stabby `->(x) { … }` (parser atom), `lambda { |x| … }` / `proc { … }` and `f.call(args)` (welcome_forms.rs); Lua
  `local` a foreign modifier; Crystal `def sq(x : Int) : Int; …; end` (end_definitions gives the Kotlin head shape);
  Kotlin `listOf`/`arrayOf`/`mutableListOf` are list literals (library_words.rs `list_constructor_calls`).
- Python f-strings `f"hi {name}"` (parser literals.rs `parse_f_string`, `brace_holes`, `{{` a brace, a note for
  `"hi \(name)"`); `return square n` parses its value at the assigned-value binding power, so a braceless call
  works; an unknown word under a cast or result type (`def g() -> str: return cube "x"`, `cube 3 as int`) is the P51
  error (casts.rs).
- C# `(int a, int b) => …` and PHP `fn($x) => …` lambdas (lambdas.rs `c_style_parameters`); typed rest parameters
  `*xs: int`, Swift `_ xs: Int...` (variadic.rs `starred`, declarations.rs partial_application guard); a lambda
  default naming an earlier parameter `(a, b = a * 2)` (lower_spaced_definitions only reads space phrases).
  `def f(x) = …` with parameters is a `:=` definition. P149: an untyped parameter used in arithmetic rules out a
  truth value, so `f x op y` is `(f x) op y`; the ambiguity stays for a body accepting anything (`same(x) := x`).
- JS `reduce(f, start)` and Swift's trailing closure after arguments `xs.reduce(0) { $0 + $1 }` (lambdas.rs
  `trailing_closure`); returned Swift closures `return { $0 + k }`: the enclosing block does not count the inner `$0`
  (`mentions_outside_blocks`), a parameter named `$0` wins over the positional `$0` (arithmetic.rs), closures.rs
  `tail` stops at a closure; Swift `-> (Int) -> Int` results and Scala's curried `def add(x)(y) = …`
  (declarations.rs `function_typed_result`, `curried_definition`); C# LINQ `Select`/`Where`/`Aggregate`/`Sum` … work
  with a note naming warp's word (welcome_forms.rs, alias rule).
- OCaml `let inc = fun x -> x + 1 in inc 4`, `fun a b -> …` (lambdas.rs `keyword_and_names`); Python lambdas
  `lambda a, b=2: …`, `lambda *xs: …`, `lambda: 42` (parser atoms.rs `parse_python_lambda`; a lambda variable with a
  rest parameter is a definition, welcome_forms.rs `starred_lambda`); Go function literals `func(a, b int) int { … }`
  and `f func(int) int` parameters. P70c: a lambda made in a loop keeps its iteration's variable.
- F# `List.map f xs` (Seq., Array.) is `map f xs` with a note, `(fun x -> …)` as an argument (only `fun` with plain
  names, Swift's `func f(x) -> Int` stays a definition); Rust type parameters `fn apply<F: …>` are skipped and P157
  generics compile untyped with a note; Dart named parameters `{required int a, int b = 2}`; English
  `To square a number: return it times it`, `square of x`, `square of x is 9` (P149), numeric `times` multiplies.
  P161: Swift argument labels (only after `func`) compile with the note "warp names a parameter once"
  (tests/functions/test_argument_labels.rs).
- `it` and `$0` belong to the innermost lambda (Kotlin): `xs.map{ it.map{ it*10 } }` shadows, a function's `it` stops
  at an inner lambda (lambdas.rs `bind_parameter`, declarations.rs `bind_it`); glued `xs.map{ it*2 }` after a dot with
  an iteration word is the call, not the data `map:{…}`. A trailing block after a method call in a definition or
  lambda belongs to the call (`f(xs) := xs.map { it*10 }`).
- Shaped parameters (wiki/argument.md, parameter_shapes.rs): `to call person{name?, phone number, mutable status} do …`
  and `f(p{name, title?})`; a missing required field of an object written at the call (`{…}`, `person{…}`,
  `person {…}`) is a compile error.

## Open
- MATLAB `@(x)`, R `sapply(c(…), function(x) …)`, `func id<T>(_ x: T)`.
- Shaped parameters: the type word of `phone number` is not checked (the wiki passes the text "899-573-5842"), nor
  `mutable`.

## Python and Ruby definitions
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
- Ruby blocks and `yield` (src/lowering/control/blocks.rs lower_trailing_blocks, after lower_spaced_definitions): a
  function whose body yields and that some call passes a block (`f {…}`, `f(a) {…}`, `f(a) do |x| … end`) gets the
  block as its last parameter `yield·block`; `yield a, b` → `yield·block(a, b)`, a block without `|params|` is
  `() => block`.
- A lambda changes the caller's variable, never a copy (P124). Inside a function it shares a cell (nonlocal_cells.rs
  `lower_lambdas`); at main level the variable becomes a `global` (`main_lambda_changes`, main runs once), and
  closures never capture a global, so `total = 0; each_twice { |x| total += x }` is 3. A closure still changing a
  captured copy after these passes is a loud error naming `global` (closures.rs `changed_capture`), never silently 0.

# Warp's type theory: the core calculus W0 and its formal model

User, 2026-10-08: "type theory is formally verifiable, let's make it highest priority to implement that". Card
type-theory. Goal: warp's type system has a formal model in Lean 4 with machine-checked soundness (progress and
preservation), and the implementation is tied to that model by tests so they cannot drift.

This note states the model and maps every rule to the code that implements it today. Where warp's checker accepts a
program the model rejects, that is a **hole** (listed at the end, each with its card); the differential test
(tests/types/test_type_model.rs) pins them, so a fixed hole fails the test until it is taken off the list.

## Why a checker of many passes can still have one theory

Warp has no single type checker. Static typing is spread over passes that each refuse one kind of mistake:
`analyzer::check_type_errors`/`check_assignment` (variable kinds), `check_kind_changes` (P45), `check_declared_types`
(`x:int = …`), `check_constants` (P130), charged `:=` (P71/P138), parameter checks in user_functions.rs (P49) and
lowering/traits.rs `admit`, and the operator checks in inference.rs `node_arithmetic`. The model W0 is the single
judgment those passes approximate. The contract: **warp rejects ⇐ W0 rejects** (soundness of the implementation is
what we want; every program W0 rejects should be refused by warp at compile time). Warp may refuse more (it refuses
ambiguous forms per notes/welcoming.md), never less, except at a listed hole.

## W0: syntax

```
types      τ ::= never | bool | int | number | text | unit | list τ | cls [C₀ … Cₙ] | any
modes      m ::= var | const | charged
values     v ::= b | n | q | "s" | ø | [] | v :: v | ref a [C…]      (lists are cons cells, as the GC $Node)
expr       e ::= v | x                          main-level name (store)
               | y                              local (parameter or let), bound by substitution
               | e + e | e - e | e * e | e < e | e == e     (`-` and `*` take numbers only; `+` also texts)
               | if e then e else e | while e do e | e ; e
               | for y in e { e }               the body once per list item, y the item; gives ø
               | e # e                          1-based element, `xs#1`
               | e .. e                         the ints from a up to b, b excluded; `a to b` is `a..b+1`
               | e ++ e                         list append, what `xs + [v]` / `xs.add(v)` lower to
               | x = e | init x e               assignment / first binding of a main-level name
               | let y : τ = e in e
               | f(e)                           call of a top-level function
               | broadcast f e                  f applied to each item of a list (warp decides it statically)
               | cast e τ                       run-time checked: the value if it fits τ, else an error
               | error "s" | try e catch e
               | new [C…] | e.f | e.f = e | e is C          instances (phase 4)
program    P ::= Σ (declared names: x ↦ (m, τ, charged body?)), Φ (functions f(y:τ):τ := e),
               Θ (classes: C ↦ its own fields f:τ), main e
```

Code: `Node` (src/node/mod.rs) carries all of this untyped; `Kind` (src/type_kinds.rs) is the run-time tag. W0 types
are the static view: `bool` is BOOL_KIND (an Int marked above the kind bits, notes/bool_type.md), `number` covers
exact decimals/rationals (Kind::Int holding a ratio, wasm_emitter/exact.rs) and floats (Kind::Float); the split into
`exact` and `float` is a refinement for a later phase. `never` is the type of `error(…)` (inference.rs
`raises_error`: "an `error(…)` branch is the bottom kind", notes/error_branch_kind.md) and of the elements of `[]`.

## Subtyping (≤)

| rule | meaning | code |
| --- | --- | --- |
| τ ≤ τ, never ≤ τ, τ ≤ any | reflexive, bottom, top (`any` = a Node, `value:any`) | — |
| bool ≤ int | true/false act as 1/0 (P195: `true + 1` is 2) | BOOL_KIND masks to Int |
| int ≤ number | `x: float = 1` is accepted | checks.rs `assignment_mismatch` (Float ← Int) |
| list σ ≤ list τ if σ ≤ τ | **covariant lists** | probes/variance/ |
| cls p ≤ cls q if q is a prefix of p | a subclass or variant extends its parent's chain | class_methods.rs inherit, traits.rs IS_TYPE |

Covariant lists are sound in warp because **lists are values**: `xs.add(v)` is the assignment `xs = xs ++ [v]` of a
new list to the variable xs (node_with_at copies; typed lists copy on `ys = xs` when either side is updated,
notes/typed_lists.md). A function receiving `names` gets the value, so TypeScript's hole (a `string[]` passed as
`(string|number)[]` and a number pushed through the alias) cannot happen: probes/variance/widening_*.wasp are all
sound. In W0 this shows up as: no expression form mutates a value, only `x = e` changes the store, and `x = e`
checks e against x's declared type. Preservation (below) is the proof.

Join (least upper bound) `σ ⊔ τ`: the type of `if … then σ else τ`, of a list literal's elements, of `try σ catch τ`.
`int ⊔ text = any`, `list int ⊔ list text = list any`, `never ⊔ τ = τ`. Code: inference.rs `branches_kind`.

## Typing Σ; Γ ⊢ e : τ (algorithmic: one type per expression, checks against upper bounds)

| expression | rule | code |
| --- | --- | --- |
| literals | `n : int`, `q : number`, `"s" : text`, `true : bool`, `ø : unit`, `[] : list never` | inference.rs `infer_type` |
| `v :: vs` | `σ :: list τ : list (σ ⊔ τ)` | `infer_list_type`, `list_type_name` |
| `x` | Σ(x) = (m, τ) ⇒ τ; reading a charged name runs its body | analyzer `Scope::lookup` |
| `e1 + e2` | `never` if a side is, `any` if a side is, `text` if a side is text (`"a" + 1` is "a1"), `int` if both ≤ int, else `number`; operands other than numbers and texts raise "not addable" at run time (checker: both ≤ number, text or any) | inference.rs `arithmetic_kind`, `node_arithmetic` |
| `e1 - e2`, `e1 * e2` | `any` if a side is, `int` if both ≤ int, else `number`; non-numbers raise "not a number" (checker: both ≤ number or any) | same |
| `e1 < e2` | ⇒ bool; non-numbers raise "not comparable" (checker: both ≤ number or any) | `infer_type` comparison arm |
| `e1 == e2` | any operands ⇒ bool (`0 == false` is true) | equality.rs |
| `if c then a else b` | c : any (truthiness: false, 0, ø, [] are falsy); a ⊔ b | `infer_type` if arms |
| `while c do b` | ⇒ unit | lowering of loops |
| `e # i` | e ≤ list any, i ≤ int ⇒ element type; out of range is an `error` at run time | `element_kind`, `index_out_of_range` |
| `e1 ++ e2` | both ≤ list any ⇒ list (elem e1 ⊔ elem e2) | list_emitter append |
| `init x e` | Σ(x) = (var or const, τ), e ≤ τ ⇒ τ | `check_assignment`, `check_declared_types` |
| `x = e` | Σ(x) = (var, τ), e ≤ τ ⇒ τ. const: refused (P130). charged: refused (P138) | `check_constants`, `check_assignment` |
| `let y : τ = e in b` | e ≤ τ, then b with y : τ | function locals |
| `f(e)` | Φ(f) = (y:σ):τ, e ≤ σ ⇒ τ | user_functions.rs P49, traits.rs `admit` |
| `f(xs)` broadcast | Φ(f) = (y:σ):τ, xs : list α, α ≤ σ ⇒ list τ (`f(x: int) := x+1; f([1])` is [2]); decided at compile time, so its own form | broadcasting |
| `cast e τ` | any e ⇒ τ; at run time the value if it fits τ, else an error | list-element-types run-time item checks |
| `error "s"` | `never` | `raises_error` |
| `try e catch h` | e ⊔ h | try lowering, P60, decision #34 |
| program | each charged body : τ under Σ; each function body ≤ its declared result with y : σ and assigns only `global` names; main typed | analyzer passes, functions2 (`global names`) |

A main-level name's declared type τ is its annotation (`x: int = …`, `xs: texts = …` = list text) or the type of its
first value (`x = 1` declares int; P45: a later `x = "a"` is refused, "x was an Int, is given a Text").
`const x = e` is mode const; `z := e` without parameters is charged (P71): its body runs at every read.

## Semantics (small step, store μ: name ↦ unset | value | charged body)

Left-to-right call-by-value. `x` steps to μ(x) (a charged name to its body, an unset one to `error "unset"`);
`x = v` / `init x v` store v and give v; `f(v)` steps to the body with y := v (substitution, so a callee works on a
copy); `v :: vs # i` gives the i-th element or `error "index out of range"`; `while` unfolds to `if`; `error` in any
evaluation position propagates to the whole expression, except under `try`: `try error s catch h → h`,
`try v catch h → v`.

Main-level names are the store; function parameters and lets are substituted values. A function may assign a
main-level name only when it is declared `global` (decisions: "mutating a main-level list in a function needs
`global`"): `FunsOk` requires every name a body assigns to be global. Broadcasting steps
`f([]) → []`, `f(v :: vs) → f(v) :: f(vs)`.

## Theorems (lean/WarpTypes)

- **Progress**: a closed, well-typed expression in a well-typed store is a value, an `error`, or steps.
- **Preservation**: a step from a well-typed (e, μ) gives (e', μ') with μ' well-typed and type(e') ≤ type(e).
- Soundness follows: a well-typed program never gets stuck; it ends in a value of (a subtype of) its type, raises an
  error value, or runs forever (`while`, a charged name reading itself).

All three are proved, with no `sorry` and only Lean's standard axioms (`#print axioms Warp.safety`: propext,
Quot.sound). Plain Lean 4 core (toolchain v4.34.1), no Mathlib. Build: `cd lean/WarpTypes && lake build` (~15 s).

| file | contents |
| --- | --- |
| Ty.lean | types, `sub` (decidable), `join`, `element`, `arith`; reflexivity, transitivity, antisymmetry, join is the least upper bound, monotonicity |
| Syntax.lean | `Expr`, `isValue`, `subst`, `Program` (Σ and Φ), `Ctx` |
| Typing.lean | `HasType` (one rule per row of the table above), `FunsOk` |
| Semantics.lean | store cells (unset, value, charged body), value operations, evaluation `Frame`s, `Step`, `StoreOk` |
| Lemmas.lean | values are closed, narrowing (smaller local types give smaller types), substitution, typing of `+`, `#`, `++` |
| Soundness.lean | `frame_typing`, `preservation`, `progress`, `Steps`, `safety` |
| Checker.lean | `typeOf` + `typeOf_sound`, `Spec.check` + `check_safe`, `elaborate` (warp's source order), `verdict` |

Model choices to keep in mind (each a simplification of warp, not a claim about it):
- Typing is algorithmic (one type per expression, no subsumption rule), so preservation says the type may shrink:
  `if c then 1 else 2.5` has type number and steps to `1 : int`.
- `==` compares values structurally; warp's `0 == false` (true) is a refinement for later.
- Run-time element checks (list-element-types stores `names = f()` with a check per item) are `cast`s: the exporter
  wraps a call stored in a declared list.

## The executable checker and the tie-in (phase 3)

- Checker.lean: `typeOf` (the algorithm) with `typeOf_sound` (it agrees with `HasType`); `Spec.check` checks a whole
  program, `check_safe`: an accepted program is safe from its initial store. `elaborate` turns warp's top-level items
  in source order into a `Spec` the way warp's analyzer does: a name's type is its annotation, else the type of its
  first value, widened over the numbers it is given later (P45: `x = 1; x = 2.5` is a number; a bool stays bool), an
  undeclared list is `list any` (list-element-types: `[1, "a"]` is a valid list); a function's result is inferred by
  iteration from `never` (recursion); a call with a list where the function takes its items is a broadcast.
- src/law/type_model.rs: `export` turns a warp program (parsed source) into W0 items; `verdicts` runs the model (it
  builds the lake project first, so a broken proof fails); `warp_verdict` is `pipeline::compile`. Reuses the law
  bridge's Lean runner (law/lean.rs). `warp types <file|code>` prints the export and both verdicts.
- tests/types/test_type_model.rs: the model builds, no `sorry`/`admit`/`axiom` in it, the soundness theorems rest only
  on Lean's core axioms; on the corpus, warp rejects every program the model rejects except the KNOWN_HOLES, and the
  model accepts what warp compiles. A fixed hole fails the test until it is taken off the list.

## Holes (warp compiles, W0 rejects), pinned by KNOWN_HOLES in tests/types/test_type_model.rs

| program | W0 | card |
| --- | --- | --- |
| `x: bool = 2`, `b = true; b = 2`, `f(b: bool) := b; f(2)` | int ≰ bool (b then prints `yes`) | bool-assign |
| `s: Shape = Circle("a", 2); s.r`, `c: Color = rgb(1, 2, 3); c.r` | Shape has no field r (warp looks it up at run time) | upcast-field |
| `s = 0; for x in 3 { s += x }; s` | a for over an int (warp fails at run time, "not a list") | non-list |

P199: a bool place (a declared variable or parameter, or a variable whose first value is evidently bool) takes
the literals 1 and 0 as yes and no. The exporter elaborates them to `.bool` (src/law/type_model.rs bool_literal),
so `x: bool = 1` and `b = true; b = 1` are no holes.

analyzer admits (the run-time check of a value against a builtin type word) agrees with W0 `Ty.sub` for every
type word × W0 scalar value type (test_warp_admits_what_the_type_model_subtypes), except bool ← int: the run-time
Kind has no bool, so any int passes a bool check there.

Fixed: declared list elements (`xs: ints = [1]; xs.add("a")`, `xs = ["a"]`, `xs: texts = [420]`), card
list-element-types (warp-a1, main 499bb5b1c); typed parameters (`f(x: text) := x; f(3)`, card param-types) and
call results (`f(x: int) := x + 1; y = f(2); y = "a"`, card call-result), functions2 on main by 18952a635.
bool-assign is on its way (warp-15 tip 64c8abf14). Not a hole: `xs = [1]; xs = ["a"]`, an undeclared list holds anything.

## Classes and sum types (phase 4)

A class type is its ancestor chain, root first: `class Circle extends Shape` is `cls ["Shape", "Circle"]`, and a
variant of `type Color = red | rgb(r: int, …)` is `cls ["Color", "rgb"]` (P178, P179: a variant is a class extending
the sum). So subtyping is the prefix order and join the common prefix, with no class table in `Ty`.
- Instances live on a heap that only grows (`Store.heap`), and a value is `ref a path`, so instances are shared:
  a mutation through a parameter is visible to the caller (P200). The ref carries its class, so typing needs no heap
  typing; `HeapOk` says every field set on an instance holds a value of the field's type.
- A field's type is the first declaration along the chain from the root (`Program.fieldTy`), so a subclass keeps its
  ancestors' field types. Fields are invariant: a write must fit the declared type, a read gives it.
- `new` makes an instance with no field set (reading one is the error "unset field"). The exporter turns a
  constructor `Circle("a", 2)` into `let o = new …; o.name = "a"; o.r = 2; o`, with fields in warp's order:
  ancestors first.
- `e is C` (warp parses it as `e == C`) is the run-time test that C is in the instance's chain. A `cast` to a class
  type is the checked downcast of a match arm.
- Weaker than warp claims: reading through a dangling reference, or a reference whose class differs from its
  object's, steps to an error rather than being ruled out. Neither can arise from a program, since `new` is the only
  source of refs.

Agreeing corpus: constructors with wrong field types, field writes, inheritance, variants, `is`, and parameters
typed by a sum or by a variant.

## Values: the executable evaluator

Evaluator.lean: `step` computes the next state, `step_sound` proves each of its steps is a `Step`, and `run` iterates
it with fuel (`run_sound`: the end state is reachable by `Steps`). `outcome` elaborates, checks and runs a program,
printing the value as warp prints it (`yes`, `[1 2]`, `"a"`) or `?` where the model keeps no value (floats are
exported as `num 0`; instances). test_warp_computes_what_the_type_model_computes compares that with warp's
`pipeline::eval` on the corpus. The known differences are listed in KNOWN_VALUE_DIFFERENCES, each with its card:
- instance-field (P200): `f(q: Point) := q.x = 7; p = Point(1); f(p); p.x` gives 1 in warp, 7 in the model.
- bool-literal-value (P199): `f(b: bool) := b; f(1)` gives 1 (the parameter keeps the int), and the assignment
  expression `x: bool = 1` gives 1 while x holds yes.
Found on the way: the exporter had mapped `-` and `*` to `+`, so W0 accepted `"a" - 1` and `f(n - 1)` recursed upward.
W0 now has `arith op a b`, which takes numbers only.

## Functions of several parameters, `def`, inferred parameters

- The exporter desugars `f(a, b) := body` (and `def f(a, b) { body }`) into a function of one parameter, an
  instance of the generated class `f·args` with fields a and b. The body reads `args.a`, and a call `f(1, "x")`
  constructs the arguments object. Per-position types come from class typing, so the proofs are unchanged. A
  function of no parameters takes `unit`; `two()` parses as `(two)`.
- Elaboration (Checker.lean `Guesses`) types an unannotated parameter or class field by the join of the values the
  program gives it (call arguments, field writes), iterated so recursive calls count. Several kinds join to `any`
  (P173); a place never given a value holds `any`. Elaboration is outside the proof: `check_safe` holds for any
  elaborated Spec.
- Gradual typing (P203: annotated code is strict, unannotated code is `any` checked at run time). `HasType` means
  "cannot get stuck": `+`, `-`, `*`, `<` and field reads type any operands, and the semantics steps operands of the
  wrong kind to an error ("not addable", "not a number", "not comparable"; a field read off a non-object "not an
  object"). Operations on `any` give `any`; a field read gives the declared field type for a class that declares
  it, else `any`. The checker (`typeOf`) stays strict: operands must be addable or numeric, where `any` counts as
  both, and a field read needs `never`, `any` or a class declaring the field (`strictRead`, P201). Corpus:
  `f(x) := x + 1; f(3); f("a")` ("a1"), `f(x) := x - 1; f(3); f("a")` (a run-time error), `x: any = 2; x * 3`.
  warp is stricter than W0 where it knows the value: `x: any = "a"; x - 1` is a compile error in warp (constant
  propagation), so that form stays out of the corpus.
- Coverage (probes/type_coverage.sh over data/types/sample_programs.txt, 599 is!() programs sampled from tests/,
  result data/types/coverage.txt): 83 were inside W0 before this phase. On 2026-10-08, 95 exported, but 23 of those
  were mis-exports the model rejected (method calls such as `x.size` or `4.square` exported as field reads, named
  arguments, overloads, first bindings inside expressions, `.add` on a text) or a model gap (`#` and `++` on `any`
  and texts). The exporter now refuses the former (a field read needs a field some class declares; `.add` needs a
  name bound to a list). Lists became gradual like arithmetic: `#` gives `elementTy` (a text's element is a text,
  anything but a list or text gives `any`), `++` takes any operands, and non-lists raise "not a list" when it runs.
  The checker demands lists, texts (for `#`) or `any`. Result: 77 exported, all agreeing with warp; with braced
  blocks as expressions (`if c {a} else {b}`) and `+=`/`-=`/`*=` exported: 98. With effect handlers and implicit casts from `any`: 102; after repairing the sample file (53 lines were UTF-8 encoded twice, `ø` stored as `Ã¸`): 112, all agreeing (2026-10-08). With `global` (below): 121; with `for` loops: 122 of 600; with function locals: 123; with `let x = …`, `shared n = …` and C-style `int i = 2` / `String s = …`: 125; with ranges: 131; with `not`/`and`/`or`: 140; with `a, b = xs`: 141; with lambdas: 149, all agreeing. Most of the rest:
  imports (`use`), multiple assignment `a, b = xs`, nested functions, maps `{a:1}`, lambdas, `i++`, `global`, string methods.

## Globals

`global n = 0` binds a main-level name functions may assign (`Decl.isGlobal`; `ProgramOk` demands a function body
assigns only globals); `global y` inside a function, or bare at main level before `y = …`, only declares. The
exporter collects every name declared `global` anywhere first. In warp a function's assignment to a name without
`global` makes a local (`n = 0; def f(x) { n = 5; x }; f(3); n` is 0): see Function locals. Elaboration runs twice so a function can read a main-level name whatever their order
(`elaborateTyped`: the second pass infers functions seeing the first pass's names). Not yet: `global float y = …`
(the exporter does not read the type word yet), `global x; x` (read before any value).

## Lambdas

`x => body` is `Expr.lam` (an expression; parameter `any`, warp's lambdas are unannotated), which steps to the closed
value `Expr.clo` once substitution has replaced the locals it captures; `f(a)` of a value is `Expr.app`, beta
reduction by substitution, "not a function" otherwise. `Ty.fn r` is covariant in its result like `list`;
`resultTy` (total: `any` for a non-function, the checker's `callable` rejects those statically). Exporter: a name
bound to a lambda is a function in warp (a bare `f` is a call missing its argument), so bare reads are refused;
warp's late binding (a function or lambda reading a main-level name changed after its definition, then called,
needs `global x`) is refused as outside W0, since W0 reads at call time. Hole lambda-reassign: `f = x => x*2; f = 3`.

## Destructuring

`a, b = xs` (first bindings only) exports as a hidden `·tupleN = xs`, its items counted by a `for` loop into
`·countN` (warp fails with "wrong number of values" otherwise), then `a = ·tupleN#1`, `b = ·tupleN#2`: the names
hold anything, as in warp. `a, *mid, z = …` is refused; destructuring into names that hold values fails in warp
(card destructure-existing).

## not, and, or

No new forms: `not e` is `if e then no else yes`; `a and b` is `if a then b else a` and `a or b` is
`if a then a else b` (warp's values: `0 and 7` is 0, `1 and 7` is 7), exported only when a is free of effects
(a literal, a name, operators on such), since it is evaluated twice.

## Ranges

`a..b` (`Expr.range`, `intList`) is the list of ints from a up to b, b excluded; `a to b` / `a...b` export as
`a..(b+1)`; a reversed range is empty. Typing: `list (arithTy ta tb)`, the checker demands number bounds. Number
bounds fail when the range runs (W0 keeps no float values; warp gives `[1.5 2.5]` for `1.5..3`), letter ranges
(`'a'..'e'`) are refused by the exporter; `"a"..3` compiles in warp and fails at run time (card range-mixed).
`add 1 to 2` of a function `to add number a to number b: …` parses as `add (1 to 2)`: the exporter splits the pair
into the two arguments, as warp does.

## Function locals

W0's locals are bound by substitution and never change; warp's function locals do (`out = []; for x in xs {
out.add(x) }`). The exporter keeps each name a function body assigns (but its parameters and globals) in a cell: a
fresh instance of a class `f·n` per call (`.letIn n (.cls [f·n]) (.new [f·n]) body`), read as `n.·value`, written by
`.set`, so recursion gets one cell per call like warp's frames, and no proof changed. `Item.cell` declares the class;
its one field `cellField` is typed like a main-level name by its first value (widened: a list holds anything), since
warp rejects `def f(){ m = 5; m = "a"; m }`. A value of type `any` written to a typed field (or cell) is checked at
run time like any other P204 place (`castDynamicValues` handles `.set`), and functions are inferred from their body
with those checks in place (`draftFunction`), else a body that only types with the check got the result `never`.

## For loops

`for y in l { b }` (`Expr.forIn`): l evaluates first; an empty list gives ø, `h :: t` steps to `b[h/y] ; for y in t
{ b }`, any other value is the error "not a list". Typing: y has the list's element type (`listElem`, `any` when the
list's type is not a list type), the loop has type `unit`; the checker also demands a list type (`listy`), so
`for x in 3` is rejected (hole non-list). The exporter refuses `it`, a type word as the variable (type filters, unit
walks), ranges, and a variable that is a main-level name (warp's loop assigns that name, so `x = 5; for x in [1] {}; x`
is 1 there; W0's y is a local). `break` in a loop is not in W0 (as for `while`). Value difference found:
`n: int = 0; for x in [1, "a"] { n = x }; n` is 97 in warp, an error in W0 (the item skips the P204 check, card
item-unchecked).

## Equality (P208)

`Expr.eq same a b`: `a == b` (same = false) is loose: two instances of one class are equal when their fields are
(`looseEq`, walking the heap with fuel for cycles; `Program.fieldNames` enumerates a class chain's fields for it),
lists item by item, anything else by value. `a === b` / `a same b` (same = true) is identity on instances (an
instance is its address); on other values it compares by value, so `0 === false` is no. Not modelled: list identity
(W0's lists are values, `[1] === [1]` is yes in W0, no in warp under P208) and `1 === 1.0` (W0 keeps no float
values). The exporter maps `!=` and `!==` to the negation; `same`/`identical` spellings once warp parses them
(card same-identity, which also covers warp still answering yes for `p === q` of distinct instances).

## Inline unions and optionals

`x: int | text`, `int or text`, `(int|text)` and the optional `int | ø` stay out of `Ty`: a union in the order would
make `join` a true least upper bound (`int ⊔ text = int | text`), else `join_mono` and with it narrowing break (with
`join int text = any`, `int ≤ int|text` but `any ≰ int|text`), and every type operator (`plus`, `element`,
`readTy`, `writeTy`) would need a union case. Warp runs operations on a union like on `any`, so W0 models a union
declaration as its alternatives' join (`Ty.joinAll`: `int | text` is `any`, `int | float` is `number`) and every
value given to it as `cast v [alternatives]`, the run-time check, which keeps a value fitting one alternative. The
checker admits a cast when the value's static type is consistent with an alternative (`consub`, gradual consistent
subtyping: `any` stands for anything, `list any` fits `list int`), so `x: int | text = 3; x = 2.5` and
`f(x: int | text) := x; f(2.5)` are rejected as warp rejects them. The exporter (src/law/type_model.rs) reads the
union with the analyzer's `union_type_name` (a covered part dropped, ø made optional) and wraps main-level bindings,
assignments and single-parameter call arguments. Not yet: unions as fields or as one of several parameters, unions
of classes (warp refuses them for now), `x as T?` (a conversion in warp: `"4" as int?` is 4; W0's cast only checks).
`x: int? = 3` does not parse in warp (card int-spaces); the corpus spells it `x: int | ø = 3`.

## Implicit casts from `any`

A value of static type `any` given to a narrower place is a run-time check, not a compile error: elaboration
(Checker.lean `castDynamicValues`, after names are widened) rewrites `x = v`, `x: T = v` and `f(v)` with `v : any`
into `x = cast v [T]` (T the name's or parameter's type), for annotated places as well, since warp admits all of them
at compile time (`y: any = 3; x: int = y`, `f(n: int) := n + 1; f(y)`, `x = xs#1` of a mixed list,
`level = event.level`). The cast is part of the proved calculus, so soundness needs nothing new. User decision P204:
an annotated place given an `any` value compiles and is checked when it runs (not a compile error). Warp does not check these values at run time yet: `y: any = "a";
x: int = 0; x = y; x` gives 97 (card int-unchecked, KNOWN_VALUE_DIFFERENCES).
`Expr.rewrite` (bottom-up, carrying the locals in scope) is the one traversal behind both elaboration rewrites
(`resolveCalls` decides broadcasting the same way).

## Effect handlers

Semantics from the functions session (notes/effect_handlers.md): `on ev {h} in {body}` (`Expr.handle`) runs body with
h as the innermost handler of ev; `emit ev payload` (`Expr.emit`) evaluates to the innermost active handler's value
(tail-resumptive, an emit is a call of the handler); the handler body runs with only the handlers outside it
(`Expr.scope k e`), so an emit inside a handler goes outward; a program-wide `on ev {h}` (`Item.on`,
`Program.handlers`) answers when no block handler is active; an emit nobody answers gives ø (`.unit`; warp warns at
compile time, P202). The payload is the local `event` (type `any`); the exporter makes each event's payloads
instances of a synthetic class `ev·event` whose fields are every key the program emits it with. The store carries
the handler stack (`Store.handlers`, innermost first; `StoreOk` = vars ∧ heap ∧ handlers ok). Typing: each event has
a result type R (`Program.effects`, inferred as the join of its handlers' types, `Guesses.effects`); a handler must
fit R, an emit has type `join R unit`. `ProgramOk` (was `FunsOk`) also demands the program-wide handlers fit.
Program-wide handlers are hoisted in both: `x = emit ask; on ask { 1 }; x` is 1.
Aborting handlers (effect handlers Step 3): `break v` (bare `break` = ø) in a block handler is `Expr.abort ev none v`.
It unwinds like an error, but `try` does not catch it (`tryAbort`); leaving the handler's `scope k` fixes the depth k
of the block whose handler ran (`scopeAbort`, the handlers outside that block); a `handle` of ev whose body aborts
with ev at depth `μ.handlers.length` ends with v, any other passes it on (`handleAbort`). Typing: `break v` has type
`never` and v must fit the event's abort type (`Program.aborts`, inferred as the join of its `break` values); a
block gives `join body aborts`. Preservation needs nothing more; progress allows a final state "an abort with a
value", since the proof does not track that every abort meets its block. The checker's `breaksPlaced` (in
`verdict`, outside the proof) keeps every `break` in a block handler of its own event and out of loops, so warp's
programs cannot reach that state. Program-wide handlers cannot abort.
Exporter limits: a first binding inside a handler or body (`on ask { y = emit ask; … }`) is refused like any first
binding inside an expression; pre-declare the name. Warp bugs found here: cards handler-annotated (an annotated
global assigned by a program-wide handler keeps its old value) and handler-global (a global assigned from an emit
inside a handler body loses the outer handler).

## Later phases

Optional and auto-unwrap (P179: `a: int = Some(3)`), payload-free variants (`red`: one shared instance per variant),
the value comparison above, errors as stored values (`r = f(-1); if r failed …`: a `τ or error` sum),
exact vs float, codepoints (`"a"` parses as one; `codepoint ≤ text` for parameters), maps, units, then tasks.

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
types      τ ::= never | bool | int | number | text | unit | list τ | any
modes      m ::= var | const | charged
values     v ::= b | n | q | "s" | ø | [] | v :: v                    (lists are cons cells, as the GC $Node)
expr       e ::= v | x                          main-level name (store)
               | y                              local (parameter or let), bound by substitution
               | e + e | e < e | e == e
               | if e then e else e | while e do e | e ; e
               | e # e                          1-based element, `xs#1`
               | e ++ e                         list append, what `xs + [v]` / `xs.add(v)` lower to
               | x = e | init x e               assignment / first binding of a main-level name
               | let y : τ = e in e
               | f(e)                           call of a top-level function
               | broadcast f e                  f applied to each item of a list (warp decides it statically)
               | cast e τ                       run-time checked: the value if it fits τ, else an error
               | error "s" | try e catch e
program    P ::= Σ (declared names: x ↦ (m, τ, charged body?)), Φ (functions f(y:τ):τ := e), main e
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
| `e1 + e2` | both ≤ number or text; `text` if a side is text (`"a" + 1` is "a1"), `int` if both ≤ int, else `number`; `never` if a side is | inference.rs `arithmetic_kind`, `node_arithmetic` |
| `e1 < e2` | both ≤ number ⇒ bool | `infer_type` comparison arm |
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

P199: a declared bool place (variable or parameter) takes the literals 1 and 0 as yes and no; the exporter
elaborates them to `.bool` (src/law/type_model.rs bool_literal), so `x: bool = 1` is no hole.

Fixed: declared list elements (`xs: ints = [1]; xs.add("a")`, `xs = ["a"]`, `xs: texts = [420]`), card
list-element-types (warp-a1, main 499bb5b1c); typed parameters (`f(x: text) := x; f(3)`, card param-types) and
call results (`f(x: int) := x + 1; y = f(2); y = "a"`, card call-result), functions2 on main by 18952a635.
bool-assign is on its way (warp-15 tip 64c8abf14). Not a hole: `xs = [1]; xs = ["a"]`, an undeclared list holds anything.

## Later phases

Sum types and enum cases with values (P178, P179: sealed classes, optional as a sum with auto-unwrap), classes with
typed fields (P131, notes/classes.md), errors as stored values (`r = f(-1); if r failed …`: a `τ or error` sum),
exact vs float, codepoints (`"a"` parses as one; `codepoint ≤ text` for parameters), maps, then effects and tasks.

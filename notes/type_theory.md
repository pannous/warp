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
| `e1 + e2` | both ≤ number; `int` if both ≤ int, else `number` | inference.rs `arithmetic_kind`, `node_arithmetic` |
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
| `error "s"` | `never` | `raises_error` |
| `try e catch h` | e ⊔ h | try lowering, P60, decision #34 |
| program | each charged body : τ under Σ; each function body ≤ its declared result with y : σ; main typed | analyzer passes |

A main-level name's declared type τ is its annotation (`x: int = …`, `xs: texts = …` = list text) or the type of its
first value (`x = 1` declares int; P45: a later `x = "a"` is refused, "x was an Int, is given a Text").
`const x = e` is mode const; `z := e` without parameters is charged (P71): its body runs at every read.

## Semantics (small step, store μ: name ↦ unset | value | charged body)

Left-to-right call-by-value. `x` steps to μ(x) (a charged name to its body, an unset one to `error "unset"`);
`x = v` / `init x v` store v and give v; `f(v)` steps to the body with y := v (substitution, so a callee works on a
copy); `v :: vs # i` gives the i-th element or `error "index out of range"`; `while` unfolds to `if`; `error` in any
evaluation position propagates to the whole expression, except under `try`: `try error s catch h → h`,
`try v catch h → v`.

Main-level names are the store; function parameters and lets are substituted values. A function cannot assign a
main-level name (decisions: "mutating a main-level list in a function needs `global`"), which W0 reflects by not
typing `x = e` inside function bodies (Σ is read-only there).

## Theorems (lean/WarpTypes)

- **Progress**: a closed, well-typed expression in a well-typed store is a value, an `error`, or steps.
- **Preservation**: a step from a well-typed (e, μ) gives (e', μ') with μ' well-typed and type(e') ≤ type(e).
- Soundness follows: a well-typed program never gets stuck; it ends in a value of (a subtype of) its type, raises an
  error value, or runs forever (`while`, a charged name reading itself).

## Holes found 2026-10-08 (warp compiles, W0 rejects)

`warp compile --wasm '<code>'` accepts all of these:

| program | W0 | card |
| --- | --- | --- |
| `xs: ints = [1]; xs.add("a")` | text ≰ int | list-element (warp-a1 is on it) |
| `xs: ints = [1]; xs = ["a"]` | list text ≰ list int | list-element |
| `xs = [1]; xs = ["a"]` | list text ≰ list int (P45 sees only "List") | list-element |
| `f(x: text) := x; f(3)` | int ≰ text: parameters are checked only for int (P49) and classes (admit) | param-types |
| `b = true; b = 2` | int ≰ bool, and the result prints `yes` | bool-assign |
| `f(x: int) := x + 1; y = f(2); y = "a"` | text ≰ int: a call result has no evident kind for P45 | call-result |

## Later phases

Sum types and enum cases with values (P178, P179: sealed classes, optional as a sum with auto-unwrap), classes with
typed fields (P131, notes/classes.md), errors as stored values (`r = f(-1); if r failed …`: a `τ or error` sum),
exact vs float, codepoints (`"a"` parses as one; `codepoint ≤ text` for parameters), maps, then effects and tasks.

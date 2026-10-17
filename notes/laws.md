# Laws: progressive verification

```warp
square(x) := x*x
law square(-x) == square(x)
```

A `law` states a property once. Its assurance rises without touching the source.

| Level | What happens | Where |
| --- | --- | --- |
| stated | `law <expr>` is split from the program, bound to the first user function it calls, and free variables get the kinds of the parameters they feed (`x:float` → Float, otherwise Int) | `law::separate_laws` |
| asserted | debug builds (`cfg!(debug_assertions)`): each law call like `square(x)` is matched against concrete program calls `square(3)`, and the instance is evaluated. A violation replaces the result with `Error("law … violated: counterexample x=3")` | `law::assert_laws`, called from `wasm_emitter::eval` |
| tested | `PROPERTY_TRIALS` (64) deterministic inputs: edge cases 0, ±1, ±2, i64::MAX, i64::MIN and ±3037000500 (the first square past i64::MAX) first, then xorshift values in ±1000. Each instance runs through the full parse → wasm → Node path (`eval_parsed`) | `law::property_test` |
| proved | pure integer functions and the law are exported to Lean 4 as unbounded `Int` and checked | `law::lean::prove` |

`law::verify(code)` runs tested → proved and returns a `LawReport` per law.
CLI: `warp verify file.warp` (or inline code) prints the reports, then `ALL PROOFS CHECK` only when every law is
proved; otherwise it prints `SOME PROOFS FAIL` and exits 1.

## Project law file and quantified alias

`LAWS.warp` at a project root is the human-owned contract: AI agents may read and verify it but must not edit it.
After changing code covered by those claims, `warp verify LAWS.warp` is the gate. Warp needs no separate generated
`PROOF.warp`: `warp verify` generates Lean obligations and checks them with Lean itself.

A law can use the compact original form or the named Bend-style alias; both become the same `Law` before assertion,
property testing, and Lean export:

```warp
square(x) := x*x
law square(-x) == square(x)

law square_symmetry:
  for x: int
  square(-x) == square(x)
```

Several typed variables are one `for` line each. The name labels reports and the generated Lean theorem; it does not
change the proposition.

## Lean export

- Warp Int promotes to arbitrary precision: `square(3037000500)` is `9223372037000250000`. It is exported as Lean `Int`, so **Proved means proved for Warp's mathematical integer semantics**. Explicit `as i64` values still wrap, but casts are not exported yet.
- Definitions use `def f (x : Int) : Int := …`. The Lean term covers `+ - * %`, `^n` with a literal nonnegative exponent, unary minus, `?:`, `if then else`, and comparisons (lifted to 0/1 inside terms). `%` uses `Int.tmod`, matching signed truncating remainder. Warp `/` yields a Float, so it isn't exported.
- The law becomes `theorem f_law (vars : Int) : prop := by try simp only [defs]; all_goals first | (t; done) | …`. Every tactic is wrapped in `; done` because `simp` can rewrite a goal without closing it and still count as success.
- Tactics are core Lean only: `rfl decide ac_rfl grind simp`, plus a small checked helper for `0 ≤ x*x`; no Mathlib is required. Unlike finite `BitVec`, arbitrary `Int` cannot be exhaustively decided. A law that generated tests do not refute and Lean cannot prove therefore remains `Tested/Unknown` rather than producing a spurious finite-word counterexample.
- Results are cached by the hash of the Lean source in `~/.cache/warp/lean/law_<hash>.{lean,result}` (shared by all checkouts). The export unfolds only the functions a law reaches (an unused simp argument is a Lean warning), and only `: error` lines fail a proof. This is the "proof result recorded" step until the semantic artifact exists.
- Not exported, so the law stays at the Tested level: Float parameters, `/`, and anything outside the term subset.

## Recursive functions

- Exported with `termination_by a.toNat + b.toNat` (the sum of the parameters' sizes) and `decreasing_by all_goals
  omega`: each recursive call must lower the sum, as `n - 1` under `¬ n ≤ 0` does. Lean refuses other recursions
  (Ackermann), and the law stays Tested.
- A law calling a recursive function with its variables is tried by functional induction first:
  `fun_induction sum_to n <;> first | grind [eqs] | simp_all [eqs] | omega`, the recursive call's value the induction
  hypothesis (Bend's "a recursive call is the induction hypothesis", done by Lean); then `grind [eqs]`; then the
  direct tactics. A recursive function is never in `simp only [...]` (its equation loops).
- `law a => b`: an implication. Instances where a is false hold without running b (`n >= 0 => sum_to(n) …`).
- An instance recursing deeper than the wasm stack (`sum_to(i64::MAX)`) is skipped, no counterexample; when every
  instance is that deep the law is Unknown.
- Proved: `sum_to(n) >= 0`, `n >= 0 => sum_to(n) * 2 == n * (n + 1)`, `e >= 0 => power(b, e + 1) == b * power(b, e)`.
- Lemmas: a law proved earlier in the program is a lemma of the laws after it (`lean::prove_using`): its theorem
  `warp_lemma_i` is stated (`by sorry`, it was proved in its own file) and handed to `grind [...]` and `simp_all [...]`.
  `law x > 0 and y > 0 => x * y > 0` (proved by `simp_all [Int.mul_pos]`) then proves `fact(n) > 0`, which alone is
  only Tested (nonlinear). simp_all never gets a recursive function's equation (it loops); grind does.
  Next: proofs as warp defs (Bend style: equality, rewrite, the recursive call as hypothesis), probes/laws/sum_to.warp.

## Verdicts

- `Proved`: Lean closed the goal.
- `FAILED … counterexample x=1`: a property test instance evaluated to false.
- `Tested (reason)`: every generated instance held, but Lean could not prove it or export it.

Counterexamples come from property tests evaluated in wasm. Lean either proves the universal integer statement or returns `Unknown`.

## Next

- Attach laws to `FunctionDecl` in `src/semantic/` once that IR exists. Persist the verdict in the Warp-serialized artifact instead of `~/.cache/warp/lean`.
- Use an SMT backend (z3 is installed) as a faster prover for linear and bitvector laws.
- Asserted mode checks concrete call sites at compile time. It doesn't yet check values computed at runtime inside the WASM module.
- Lawful lifting: broadcasting is allowed only where functor laws are Proved.
- Beyond Lean: proofs as warp functions (Bend style) and an own kernel proved sound in Lean: notes/own_prover.md;
  Bend's laws and runtime: notes/bend.md.

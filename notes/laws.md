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
CLI: `warp verify file.wasp` (or inline code) prints the reports and exits 1 if any law is violated.

## Lean export

- Warp Int promotes to arbitrary precision: `square(3037000500)` is `9223372037000250000`. It is exported as Lean `Int`, so **Proved means proved for Warp's mathematical integer semantics**. Explicit `as i64` values still wrap, but casts are not exported yet.
- Definitions use `def f (x : Int) : Int := …`. The Lean term covers `+ - * %`, `^n` with a literal nonnegative exponent, unary minus, `?:`, `if then else`, and comparisons (lifted to 0/1 inside terms). `%` uses `Int.tmod`, matching signed truncating remainder. Warp `/` yields a Float, so it isn't exported.
- The law becomes `theorem f_law (vars : Int) : prop := by try simp only [defs]; all_goals first | (t; done) | …`. Every tactic is wrapped in `; done` because `simp` can rewrite a goal without closing it and still count as success.
- Tactics are core Lean only: `rfl decide ac_rfl grind simp`, plus a small checked helper for `0 ≤ x*x`; no Mathlib is required. Unlike finite `BitVec`, arbitrary `Int` cannot be exhaustively decided. A law that generated tests do not refute and Lean cannot prove therefore remains `Tested/Unknown` rather than producing a spurious finite-word counterexample.
- Results are cached by the hash of the Lean source in `~/.cache/warp/lean/law_<hash>.{lean,result}` (shared by all checkouts). The export unfolds only the functions a law reaches (an unused simp argument is a Lean warning), and only `: error` lines fail a proof. This is the "proof result recorded" step until the semantic artifact exists.
- Not exported, so the law stays at the Tested level: recursive functions (Lean would need termination proofs), Float parameters, `/`, and anything outside the term subset.

## Verdicts

- `Proved`: Lean closed the goal.
- `FAILED … counterexample x=1`: a property test instance evaluated to false.
- `Tested (reason)`: every generated instance held, but Lean could not prove it or export it.

Counterexamples come from property tests evaluated in wasm. Lean either proves the universal integer statement or returns `Unknown`.

## Found by laws

- Laws caught a real bug: in `:=` functions, `x:float` parameters are compiled as Int, so `half(x:float) := x/2; half(1.0)` returns 0. See the ignored test `test_law_float_parameters_are_tested`. The cause is that `UserFunctionDef.params` keeps only defaults, not types, and `infer_function_return_kind` assumes Int.

## Next

- Attach laws to `FunctionDecl` in `src/semantic/` once that IR exists. Persist the verdict in the Wasp-serialized artifact instead of `~/.cache/warp/lean`.
- Use an SMT backend (z3 is installed) as a faster prover for linear and bitvector laws.
- Asserted mode checks concrete call sites at compile time. It doesn't yet check values computed at runtime inside the WASM module.
- Lawful lifting: broadcasting is allowed only where functor laws are Proved.

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
| proved | pure integer functions and the law are exported to Lean 4 as wrapping `BitVec 64` and checked | `law::lean::prove` |

`law::verify(code)` runs tested → proved and returns a `LawReport` per law.
CLI: `warp verify file.wasp` (or inline code) prints the reports and exits 1 if any law is violated.

## Lean export

- Warp Int is a wrapping i64: `square(3037000500)` is `-9223372036709301616`. It is exported as `BitVec 64` with signed order (`BitVec.slt`/`sle`) and truncated remainder (`BitVec.srem`), so **Proved means proved for Warp's machine semantics**. An earlier version exported Lean's unbounded `Int` and wrongly proved `square(x) >= 0`.
- Definitions: `def f (x : BitVec 64) : BitVec 64 := …`. The Lean term covers `+ - * %`, `^n` with a literal n (wraps too), unary minus, `?:`, `if then else`, and comparisons (lifted to 0/1 inside terms). Warp `/` yields a Float, so it isn't exported.
- The law becomes `theorem f_law (vars : BitVec 64) : prop := by try simp only [defs]; all_goals first | (t; done) | …`. Every tactic is wrapped in `; done` because `simp` can rewrite a goal without closing it and still count as success.
- Tactics: `rfl decide ac_rfl grind simp bv_decide`, all in core Lean (`import Std.Tactic.BVDecide`), with no Mathlib. `grind` proves ring identities over BitVec, and `bv_decide` bit-blasts the rest. `bv_decide` goes last, so when a law is false its counterexample is the reported error. That becomes `Violated("lean counterexample x=…")`, converted to the signed value.
- Results are cached by the hash of the Lean source in `target/lean/law_<hash>.{lean,result}`. This is the "proof result recorded" step until the semantic artifact exists.
- Not exported, so the law stays at the Tested level: recursive functions (Lean would need termination proofs), Float parameters, `/`, and anything outside the term subset.

## Verdicts

- `Proved`: Lean closed the goal.
- `FAILED … counterexample x=1`: a property test instance evaluated to false.
- `Tested (reason)`: every generated instance held, but Lean could not prove it or export it.

Counterexamples come from property tests (evaluated in wasm) or from Lean's `bv_decide`.

## Found by laws

- Laws caught a real bug: in `:=` functions, `x:float` parameters are compiled as Int, so `half(x:float) := x/2; half(1.0)` returns 0. See the ignored test `test_law_float_parameters_are_tested`. The cause is that `UserFunctionDef.params` keeps only defaults, not types, and `infer_function_return_kind` assumes Int.

## Next

- Attach laws to `FunctionDecl` in `src/semantic/` once that IR exists. Persist the verdict in the Wasp-serialized artifact instead of `target/lean`.
- Use an SMT backend (z3 is installed) as a faster prover for linear and bitvector laws.
- Asserted mode checks concrete call sites at compile time. It doesn't yet check values computed at runtime inside the WASM module.
- Lawful lifting: broadcasting is allowed only where functor laws are Proved.

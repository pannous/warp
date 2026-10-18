# An own prover: absorbing Lean and Bend into warp

Can warp absorb the ideas of Lean and Bend and do everything itself? Yes, in a limited form: the core, not the
ecosystem.
Background: notes/bend.md (what Bend 2 and HVM are), notes/laws.md (warp's laws today).

## Tactics vs proof terms

- Lean: proofs mostly by tactics (`simp`, `omega`, `grind`), commands that search for the proof term.
- Bend 2: no tactics; a law is a type, a proof is an ordinary `def` (`{==}` = both sides compute equal, a recursive
  call = induction hypothesis). Gains: one language for code and proofs, a small trusted kernel (`bendtt.lean`, with
  a soundness proof), fast checking (no search), proofs that don't break when automation changes. Price: long proofs,
  which Bend expects an AI to write.
- warp keeps both: tactics/SMT prove the simple laws automatically; proof functions are the fallback where automation
  fails (recursive functions).

## Small enough to build ourselves

- **Kernel**: only answers "is this proof correct?". Bend's kernel with its soundness proof is ~3.9 k lines; a warp
  kernel for the pure subset (ints, texts, lists, records, recursive functions) is months, not years.
- **Proofs as warp functions** (Bend style): law = type, proof = function, induction = recursive call.
- **Laws in the language**: `law`, generated tests and counterexamples exist; Bend's LAWS/PROOF file convention on top.
- **Simple automation**: evaluate both sides, linear arithmetic, case splits over finite values (each a few hundred
  lines).

## Not worth rebuilding

- Mathlib and the mathematical library (decades of person-years).
- Lean's elaborator and tactic machinery: most of the benefit comes cheaper from SMT (z3 is installed).
- Trust: a homegrown checker with a bug proves false things unnoticed. Remedy, as Bend does: prove warp's kernel sound
  once in Lean.

## Path

1. Today: laws are stated, tested with 64 inputs, proved by Lean (notes/laws.md).
2. Next: z3 as the fast prover for arithmetic laws, Lean as fallback.
3. Then: proofs as warp functions for recursive laws, checked by a small warp kernel on the pure subset, cross-checked
   by Lean while the kernel is young.
4. Finally: a soundness proof of the warp kernel in Lean; warp then checks its own proofs and needs Lean only for that
   one proof.

Result: laws, counterexamples, automatic proofs, hand- or AI-written proofs as ordinary code, all checked by warp, while
warp keeps mutation, floats, unbounded ints and the browser (Bend 2 gives those up).

## Prior art: self-hosted and independent kernels

- **lean4lean** (Mario Carneiro, 2023–): the Lean 4 kernel reimplemented in Lean, checks all of Mathlib at 20–50 %
  slower; correctness is proved only in parts. Its consistency proof doesn't cover inductive types yet, so it shared
  the official kernel's soundness bug #14576 (nested-inductive projections, fixed in Lean v4.32.2, 2026-07). Lags
  recent toolchains (lean4lean#17). This is the route of step 4, and it shows the expensive part: inductives.
- **nanoda** (Chris Bailey, Rust): an independent Lean 4 checker; it rejected the #14576 exploit that both the
  official kernel and lean4lean accepted. Independent reimplementation catches what a port inherits.
- **MetaCoq** (Coq kernel verified in Coq) and **Candle** (HOL Light kernel verified in CakeML) are the precedents
  for a fully proved kernel; both took several person-years.
- Lesson for warp: keep the kernel tiny and its datatypes few, prove it sound in Lean, and cross-check with an
  independent checker rather than a port.

## The real hurdle: a precise semantics

Proofs need an exact meaning for every construct; warp has many words, aliases, automatic conversions and lowering
passes. So proofs apply only to a bounded pure core (the semantic IR planned in notes/laws.md "Next"). Drawing that
boundary is the first decision, before any kernel code.

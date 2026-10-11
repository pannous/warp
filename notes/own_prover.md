# An own prover: absorbing Lean and Bend into warp (2026-10-11)

User question: "do you see any world in which we absorb the ideas of Lean and Bend into warp and just do everything
ourselves?" Answer: yes, in a limited form: the core, not the ecosystem.
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

## The real hurdle: a precise semantics

Proofs need an exact meaning for every construct; warp has many words, aliases, automatic conversions and lowering
passes. So proofs apply only to a bounded pure core (the semantic IR planned in notes/laws.md "Next"). Drawing that
boundary is the first decision, before any kernel code.

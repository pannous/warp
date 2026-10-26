# Warp Signals handover (Oct 23, 2026, ~15:10 CEST)

Card: pure-class-operators (instance operators dispatching to methods on the pure pipeline). Not on the board: the
board token lacks addProjectV2DraftIssue, so it sits in /workspace/cow/warp/todo.md "## Fallback" (uncommitted,
awaiting Supervisor's `todo import`), next to pure-units-quantity-sum.

## State
- Tree /workspace/cow/trees/classops (boxtree), branch classops, pushed to origin. Tip: dd4744878 (WIP, NOT ready: rebuild self red).
- Based on Warp Classes' pure-class-methods-3b: merged bab08c5c0 (83a587629) and 38bf42369 (8cbac33d6). 3b is not on
  origin, so pushing classops published 3b's commits too.
- Commits: c3b3a5b20 test (tests/types/test_pure_class_operators.warp, 11 value-only cases), 172d5d0e4 the first
  pass-level version (routed every operator key; broke `rebuild self`), a502bfcf5 pulled that back to the emitter guard,
  dd4744878 fix (the operator symbol is a codepoint: dispatcher name "${symbol}·operator").
- Design (Supervisor-approved emitter-guard variant):
  - core/class_methods.warp (+65/-15 vs 3b): a method named by an operator (glyph, entry word or word alias of
    an lib/aliases.warp entry with `operator:`, read via operator_word) is a routed method; one dispatcher per operator,
    `+·operator(self: any, o: any)`, tests each owner (deepest first, so inherited through lineage) and falls back to
    the builtin `self + o`. routed() is back to 3b's.
  - core/emitter/operators.warp (+4/-0): in operation_code, a boxed ("node") left operand with
    `+·operator` in module.functions calls it; skipped inside the dispatcher itself (its fallback stays builtin).
- Scratch: /workspace/classops-probe/ (STATUS.md, cases.py harness, gates.sh/gates.log, base/ = 3b 38bf42369 lib with
  its data/compiler.wasm, perf/ programs).

## Gates
Tip pushed: dd4744878 (WIP, NOT ready).
- (a) rebuild self: RED with my change: `compiler.warp compiled by itself: the pure lane refuses it: outside the core:
  a list where a text is needed` (tree, 349 s). Baseline (3b 38bf42369 alone, /workspace/classops-probe/base) rebuilds
  self green (425 s). So it is this change: compiler.warp has an operator method (time_arithmetic.warp:40
  `Span.plus(o: Span)`), so the self compile gets `+·operator` and every boxed-left `+` of compiler.warp goes through
  it; some result loses its kind (likely the dispatcher's result kind or its any-typed parameters widening a caller).
  Not debugged further (wrap-up order). Next step: dump the kinds the emitter gives `+·operator` in the self compile,
  or restrict the dispatcher's result kind to node.
- (b) hello compiled by the tree's compiler.wasm vs the baseline's: byte-identical (1702 bytes;
  `warp <root>/data/compiler.wasm -- hello.warp out.wasm <dir> <lib>`). Self-lane bytes not compared (no tree self).
  Note: `warp pure` doesn't write ~/.cache/warp/last.wasm; don't use that file for comparisons.
- (c) a program without operators (perf/plain.warp, 3M-iteration int loop) compiles to byte-identical wasm, ~1.0 s
  both. With an operator class (perf/operator.warp, ints only) ~1.04 s vs ~0.95 s base (noise level; the int `+`
  stays builtin). perf/span.warp (instances in the loop) failed with a cast before the codepoint fix; not re-timed.
- pure `warp pure hello` prints Hello on the tree.

## Counts (warp pure, cases.py, last value printed)
- test_class_operators.pairs.warp 3/8 -> 8/8; test_pure_class_operators.warp (new) 2/11 -> 10/11;
  test_class_operators.warp 2/14 -> 3/14; 3b's test_pure_class_* unchanged.
- types/ sweep (first, pass-level version, 1787 cases): 19 fixed, 3 new failures, all cured by the any-typed
  dispatcher; not re-run for the emitter version (wrap-up order: no optional sweep).

## Left over
- Operators defined outside a class (`+(a: Vec, b: Vec)`, `plus(a: Vec, …)`, `infix operator +`): not routed.
- Comparison operators (Quantity's less/more/equals): need rows in lib/aliases.warp (warp-signatures' file).
- Inherited operators (`U extends V`): blocked on 3b's emitter refusal "a test of the type V, which has cases or
  subclasses" (same as 3b's own is_parent tests).
- `+=` on instances not routed; a number on the left (`3 * v`) keeps the builtin (dispatch is on the left operand).
- Classes with an operator and an instance kept in a variable (`a = V(1, 2); (a * 3).y`) fail before and after with
  "an instance of V, a class with methods" (3b's lifting), as do the Python/Kotlin forms.
- Quantity sum: card pure-units-quantity-sum (Fallback): `use units; quantity(5,"m") + quantity(3,"m")` fails with
  "unknown name for" even as `.plus()`; not the operator dispatch.
- todo.md Fallback entries pure-class-operators and pure-units-quantity-sum await `todo import`.
- pure-compiles-compiler overlap: its "a list where a text is needed" (upper/lower) is a runtime global `= []` typed
  as text in emitter/host.warp runtime_global (its fix 3b14d49a3); a different cause from the same message here,
  which came from the pass-level routing boxing every `+` of compiler.warp (Span.plus in time_arithmetic.warp).

## on / go / channels plan (on hold until classes and operators work)
- Goes in core/extensions/concurrency/, written in Warp, no passes.
- Coverage: `on` in 159 tests, `go` in 111.
- Card order: on (folding hAkytI), hBlrU8, hBeBpI + hBlnt4, hBS72c, hBeDT8.

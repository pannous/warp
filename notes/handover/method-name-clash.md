# Handover: method-name-clash (warp-fixer, 2026-10-27, parked)

Branch method-name-clash: `spelled_members` splits fields a line joins by `;` (`names = []; counts = []` beside
methods on their own lines). Before it, such a class had a field ø, so `lifts` refused it and it stayed unlifted
(test/types/test_class_members_joined.warp shows it). Not landable alone: Counter of test_class_method_changes.warp
then lifts, its `add(x)` (an alias of +) is routed, and the list's own `names.add(x)` becomes `add·method(names, x)`,
whose fallback is the builtin `names + x`: line 28 goes from passing to failing.

## What was tried (notes/handover/method-name-clash.guard.diff)
routed() keeps a call on a value not an instance as written: `if x is C then m·method(x, …) else x.m(…)`, the
dispatcher losing its dead fallback. That fixes Counter (both lines), but the else branch is compiled with the
receiver's static type: `v = V(); v.max(3)` with `max(x)` in V fails ("the operator . on int"), since the else
branch's `v.max(3)` on a V has no method any more. A `written_call_fits` check (drop the written form when no
definition of the name takes it, list.warp's tally(xs) beside tally(x)) was added on top.

## The open question (why it is bigger than a fixer card)
The meaning of `x.m(…)` on a value of no class depends on the sugar rules (`$xs.add($v…) => $xs += [$v…]`), which run
after class_methods routes the calls (rewrites.warp is_type_declaration). The general fix routes so that the sugar
still sees the written method form on other values, without compiling a dead branch against the class's static type:
e.g. the dispatcher's fallback in method form on `self: any` once sugar applies to it, or routing after the sugar.
Also found: `use "list"; print(1)` traps on main (card use-list-traps).

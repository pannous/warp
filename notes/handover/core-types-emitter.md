# Handover: core-types-emitter (session warp-text-field), 2026-10-27

Branch `emitter-kinds` (pushed, worktree /Volumes/Worktrees/case-enum-carry), 1 WIP commit ahead of main (0e840d1e32).

## Landed on main today
- Emitter kinds step B: ValueKind cases throughout the emitter (5169d34d71); a spelled enum's ·text gives text?
  (ø == K.Text is no); seed 1ec0b599 published.
- lambdas.warp tuples as classes Iteration / Definition / InlinedFunction (81f04156df).
- instance_field_at dispatcher (45a41fcd2d; warp-fixer extended it in 1e8dcd512c).
- expected-type-names part 1: sleep(3ms) uses units on demand (0f11fe4fa1, prelude_words.warp with_expected_type_uses).
- class-text-coercion: str(x) and x as text through a class's text() (c0494668e6, card done).
- Op·text finds a case by same_node first (a14f27c74b; reaches the compiler with the next seed, seeds on hold).

## On the branch, not landed (WIP commit 0e840d1e32)
1. card enum-case-param: plain_enums.warp `enum_results` (function → enum it declares as result, read from
   definitions.warp's `{the result: E = …; the result}`), used by type_of and held_spelled: `f(Kind.List) as text`
   is "list", `g() == "b"` yes. Built, gate tier 1 green, new is! line at the end of test/types/test_spelled_enum.warp.
   A test/types comparison against main showed no real new failures (a first run under load timed out en masse).
2. expected-type-names: phrase calls too, `sleep 50ms` (samples/advanced/snake.warp:88, samples/loops/game_of_life.warp:67):
   with_expected_type_uses accepts bracket "" groups. NOT built yet; if `sleep 50ms` parses as [sleep, 50, ms] the
   argument index is off and it needs the phrase's arguments joined first.

## Next step
In the worktree: `git merge origin/main`, `scripts/rebuild.sh pure <tree>`, try `sleep 50ms; 1` and the
enum-case-param lines, smoke + `scripts/self-check.sh`/promote-self + `scripts/gate.sh --tiers 1`, push to main,
`todo done enum-case-param <commit>`. Then part 2 of expected-type-names (fix-it `use units` on `x = 3ms`).
Seed refreshes are on hold (supervisor) until the instance runtime is fast.

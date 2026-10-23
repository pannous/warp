# Retiring the hybrid lane (card retire-hybrid-lane)

## What the hybrid lane is
`WARP_SELF_HOSTED=1` (and `front`) runs parser.warp and the five twinned passes in warp (src/compiler/self_hosted.rs
TWINNED_PASSES: interpolation, match_arms, arrow_result_types, sugar_phrases, number_words), the other passes in
Rust, then analyzer.warp and emitter.warp. A program emitter.warp refuses there ("outside the core: …") goes on to
Rust's emitter.

It is not only a measuring mode: `WARP_SELF_HOSTED=pure` (`warp pure`) compiles a program the pure lane refuses
("outside the core: pure: …") "the usual way" with the switch still on (self_hosted.rs pure_module, `reads()`), which is
the hybrid lane. So every program outside the pure core runs hybrid, and every form moved from the parser or a
twinned pass into core/sugar drops out of it (decision sugar-hybrid-lane): elvis' non-name `?:` (1 test),
broadcasting (22 tests), a non-name `?:` inside a class with methods printing `0 ?: 7` silently (card
hybrid-silent-elvis).

## How much falls back (probes/hybrid_survey.sh, 2026-10-23, main bd5272362a)
Every 4th test pair line, 2060 programs: 1330 ran pure (65%), 720 were refused by the pure lane and ran hybrid, and
396 of those were refused again by the hybrid lane's emitter and ran on Rust's emitter. Refusals by kind
(data/hybrid/survey-20261023-0506.txt has the full list):

| refusals | kind | examples |
|---:|---|---|
| 381 | an unknown name: a library or sugar word the pure lane doesn't know yet | `on`, `match` as a word, `now`, `go`, `def`, `switch`, `whenever`, `undo`, `km` |
| 249 | a call of a runtime word not built in warp yet | `instance_copy`, `prelude·alike`, `dir`, `values_rough`, `list_become`, `signal_new`, `sin`, `parse` |
| 132 | a form: a group, data, a number, an assignment shape | `the group (…`, `a data`, `a value after an assignment`, an element of a text assigned |
| 111 | an operator on a kind | `≈ on int`, `as on text and node`, `> on bool and int`, `∛` with nothing before it |
| 243 | other | classes with methods, `global` declarations, a function value in `.`/`#`, running a node as code |

## Ways to retire it
- A. pure falls back to plain Rust: when the pure lane refuses, compile with the switch off (one change in
  self_hosted.rs, pure_module's caller). `warp pure` then means warp or all-Rust, never the mix; the hybrid lane is
  reached only by an explicit `WARP_SELF_HOSTED=1`, which can then go with the self-hosted suite counts it feeds
  (scripts/self-hosted-suite.sh, data/self_hosting/suite_counts.txt), and the twins with it. Every hybrid cost so far
  disappears at once, and the parser and twinned passes are free to move to core/sugar (subscripts and negative
  bounds, test words, trailing guards, the name `?:`, match_arms). Cost: a Rust edit (the Rust compiler is frozen
  save for bootstrap and crashes, so it needs the user), and the suite counts stop measuring the warp front end
  under Rust's back end.
- B. close the refusals first: the 720 pure-lane refusals of the sample (35% of programs), then delete the lane.
  Most are breadth (library words, runtime calls), not design; it is weeks of fleet work, and the sugar migrations
  wait for it.

Recommended: A now (it is the decision the migrations need), and B continues as the pure lane's own coverage work,
measured by probes/hybrid_survey.sh instead of the hybrid suite counts.

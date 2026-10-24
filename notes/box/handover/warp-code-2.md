# Warp Code 2 handover (Oct 23, 2026, 14:40 CEST)

Parked at the user's request. No running jobs of mine. Nothing new accepted.
origin/main at handover: 4116625b4.

## Landed on main
- sugar-preposition-heads 4ed2228a0a, sugar-for-loops 2be30a7815, rewrite-zip-runs-2 5f3dee2498 (engine F11/F12), parser-2.
- Still failing on main: test_rewrite_engine line 138 `doubled` (`unknown name x at 1:46`).
- Open bug (Warp Code's area): a where clause with the place word first (`where $w is parameter and label`) loses `x`. Workaround: put the place word last.

## Open branches (pushed, not merged)
| Branch | Tip | State |
|---|---|---|
| pure-harness-loose-equal | f78a5e6731 | src/compiler/pure_tests.rs +40/-2. Compares cases the way `is!` did: an int equals a float of the same value, a 1-char text equals its char. Rust test infrastructure, cleared by Supervisor. NOT measured, NOT readied: needs a box build (CARGO_TARGET_DIR=/workspace/target-harness via box-slot), before/after pure-suite counts (expected about 196 cases: /workspace/harness196.json, /workspace/harness-cases), then `todo ready` WITHOUT --express. |
| pure-wrong-type-word-prefix | 6fa54c4cac | lib/sugar/calls.warp, about 19 rule lines plus 5 probes (`int 3.7` == `int(3.7)`), merged with main at 11:40. Supervisor OK'd it as a small fix. Needs the box self-check and the 40-test before/after (/workspace/tw_tests.txt; tw-base/tw-branch jobs were killed), then `todo ready ... --express`. |
| pure-match-guard-arms | 658c010a66 | Match step A: lib/compiler/node_matches.warp +36/-4, guard arms `p if c` and bare-name binding arms. UNTESTED (never got a rebuild slot). Probes: /workspace/probes-mg/. Known gap: no scope info, so an already-bound name used in the body binds instead of comparing. Risk: `guarded_arm` uses a `test == yes` marker. Card note was never posted (the board token lacks addProjectV2DraftIssue): /workspace/pure-match-guard-arms-todo.diff. |
| fix-in-lib | b161aa32a9 | warp fix/beautify in lib/fix. No src/ diff; waits on launcher routing. |

## Match plan (approved; Supervisor plus the user for C)
Root cause: `matched` in node_matches.warp lowers a match only if EVERY arm is a value, a type word, a node case or the default. Otherwise the whole match stays unlowered, and the pure lane refuses it as `unknown name match`. 0 of 186 match tests pass; 92 of those timed out at 60 s on the busy box and need re-checking.
- A (guard/binding arms): branch above.
- B: new lib/sugar/patterns.warp in the sugar list in rewrites.warp. Rules for ranges, comparison arms, typed binding / type alternatives, value alternatives, all onto A's guard. About 25 tests.
- C (user-approved, inside node_matches.warp; split if over about 180 lines): lists/tuples (length, indexes, rest), maps (key present), constructors/enums with data (`Circle(r)`, `Some(x)`; values already work), class destructuring (`P{x, ..}`). About 90 tests.
- D: text prefix/suffix patterns (`"0x" + rest`), about 40 lines, 13 tests.
- E: code-quote patterns, about 37 tests. A separate card later; not mine.
- Don't touch lib/sugar/arms.warp (warp-layers). welcome_forms.warp is gone (c4438b97b8).

## Other notes
- The 104 "a data" pure refusals are time/date/duration values (compiler.warp:44). They belong to pure-time-arithmetic-wiring (Warp Code 3); that card still needs a note with the count.
- Handed off: on/go to Warp Signals; function values and closure cells to Warp Idioms (now shut down, so they went back to my queue: pure-call-function-values, pure-closure-cells [plan to Supervisor first], pure-text-case-words). Unowned again.
- User rules (common.md 928b79021, 9563462da): no new pass or lib/compiler file without the user's yes; lib/compiler edits over about 40 lines or new cases need Supervisor's OK first.
- Box tools: /workspace/selfcheck-branch.sh <branch> (via /workspace/box-slot.sh; per-branch lock and binary copy; uses /workspace/cow/sol-warp-latest). /workspace/build-fresh-warp.sh rebuilds that binary from main (smoke test runs with WARP_MODULE_CACHE=off). sol-warp-latest -> sol-warp-d21817966, which can't rebuild main with the cache off (`index needs a value for parameter x`; Warp Classes / local-shadows-lib-function).
- Reports: /workspace/pure-refusals-6b7abc92de.md, /workspace/pure-wrong-values-6b7abc92de.md.
- No `cowtree` on the box; git worktrees are in /workspace/cow/trees/.

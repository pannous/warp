# Task: TODO sweep (2026-09-29) — supervisor board

Baseline: ./test.sh → 611 passed, 0 failed, 132 ignored. Issue #2 closed (server file restored).
The supervisor session owns this board and assigns ONE task per agent at a time; agents do not edit the board.
Status: open | assigned <session> | review | done <commit> | parked (reason)

## High — ignore "next" / known wrong results
| id | task | status |
|---|---|---|
| A1 | tests/test_law.rs:52 `:=` functions compile `x:float` params as Int, `half(1.0) == 0` | done 768e8a87 (bundled into C2's commit by mistake)
| A2 | tests/probe_footguns.rs:129 `(x*x) as i64` panics inside a function body | assigned warp-numeric (resent) |
| A4 | footguns: `1/4+1/4` → 0 (sum typed Int) — verify first | open |
| B4 | footguns: `f := it*10; 1 + f 3` → 31; `x=[1 2 3]; x[3]` returns program text — verify first | done 872063f4 0b795bf9 (stale, already fixed; duplicate tests sent back) |

## Medium
| id | task | status |
|---|---|---|
| B1 | tests/test_functions.rs:139/145/151 `:=` vs newline precedence | done f4b601c0 (already worked, newline tests added) |
| B2 | tests/test_angle.rs:35 function application without parens precedence | done 872063f4 (already worked) |
| B3 | tests/test_string.rs:221 statement sequences `'hello';(1 2 3 4);10` → 10 | done de18c3bb — heuristic rule, needs user confirmation (1;(2 3);4 → 4 vs wiki/list.md:103) |
| A3 | tests/test_math.rs:99, :287 ignore "soon" (mixed-type variables, automatic casting) | open |
| C1 | src/main.rs:208 compile-only CLI path writing the .wasm | done bc52e71c 1d260145 (after one review round) |
| C2 | src/node.rs:412 `Text("TODO: …")` placeholder → real implementation or loud error | done 768e8a87 (Node::todo has no callers) |

## Low
| id | task | status |
|---|---|---|
| C3 | tests/test_node.rs:7 Node `remove`; tests/test_node_operators.rs `Meta` not exported | Meta: not applicable (old API gone, comment kept); remove: needs `mut` in existing test → user decision |
| B5 | semicolon binds loosest: a block (top level, {}) yields its last ;/newline item; (), [] and data keep all items (wiki/list.md); replaces de18c3bb heuristic. Step 1: survey conflicting tests | survey done (notes/semicolon_survey.md), waiting for user: 3 conflicting tests, {a:1⏎b:2} |
| C4 | tests/test_string.rs:143/150 string operator overloads (`"a".s() + 2`) | assigned warp-library |
| C5 | src/meta.rs:9 conditional compilation | open |
| L1 | test_web.rs:43 `$b.ok` emitAttributeSetter; :71 Externref kind | open |
| L2 | test_types.rs ignored type-system/generics tests | parked (large design work) |

## Rules for everyone
- Read CLAUDE.md first. Run ./test.sh before and after each fix; a fix is done only when the formerly ignored/commented test passes.
- NEVER modify or delete existing tests. You MAY remove an `#[ignore…]` attribute or uncomment a line marked TODO once it passes;
  replace that TODO word with DONE, keep the rest of the text identical.
- New tests go into tests/ (one per feature), experiments into probes/. No mock tests.
- Code style: no duplication (extract functions for repeated >2 lines), meaningful names, no trivial comments,
  constants at top of file, no ad-hoc special cases — find the general rule. Remove debug prints when done.
- Git: stage only your own files by explicit path, never `git add -A`/`commit -a`; `git pull --rebase` before push;
  never reset/stash/revert others' work, never cargo clean. Conventional commits (fix:, feature(minor):, refactor:, test:),
  no AI attribution/co-author lines. Commit + push each fix separately, then run `cargo fix --allow-dirty` and commit again if it changed something.
- Commit test_results.txt only if your change caused its diff.
- If a break comes from another agent's in-progress edit, don't fix it — note it and continue.
- If something can't be solved in reasonable time, record the difficulty in notes/footguns.md or a GitHub issue (gh) and move on.
- When done: append a short summary per item (fixed / left open + why) to the bottom of this file under "## Results", commit, push.

## Results

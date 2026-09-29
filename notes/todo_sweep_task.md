# Task: TODO sweep (2026-09-29) — supervisor board

Baseline: ./test.sh → 611 passed, 0 failed, 132 ignored. Issue #2 closed (server file restored).
The supervisor session owns this board and assigns ONE task per agent at a time; agents do not edit the board.
Status: open | assigned <session> | review | done <commit> | parked (reason)

## High — ignore "next" / known wrong results
| id | task | status |
|---|---|---|
| A1 | tests/test_law.rs:52 `:=` functions compile `x:float` params as Int, `half(1.0) == 0` | done 768e8a87 (bundled into C2's commit by mistake)
| A2 | tests/probe_footguns.rs:129 `(x*x) as i64` panics inside a function body | done a0b29543 (ignore was stale, already fixed earlier) |
| A4 | footguns: `1/4+1/4` → 0 (sum typed Int) — verify first | done e5934b38 (already fixed; fixed float locals in function bodies instead) |
| B4 | footguns: `f := it*10; 1 + f 3` → 31; `x=[1 2 3]; x[3]` returns program text — verify first | done 872063f4 0b795bf9 (stale, already fixed; duplicate tests sent back) |

## Medium
| id | task | status |
|---|---|---|
| B1 | tests/test_functions.rs:139/145/151 `:=` vs newline precedence | done f4b601c0 (already worked, newline tests added) |
| B2 | tests/test_angle.rs:35 function application without parens precedence | done 872063f4 (already worked) |
| B3 | tests/test_string.rs:221 statement sequences `'hello';(1 2 3 4);10` → 10 | done de18c3bb — heuristic rule, needs user confirmation (1;(2 3);4 → 4 vs wiki/list.md:103) |
| A3 | tests/test_math.rs:99, :287 ignore "soon" (mixed-type variables, automatic casting) | done 0768b809 (ignores were stale) |
| C1 | src/main.rs:208 compile-only CLI path writing the .wasm | done bc52e71c 1d260145 (after one review round) |
| A5 | float local read in an exact context truncates silently (emit_truncated_float) — loud would break existing tests | survey fd9dad15 (notes/float_truncation_survey.md): 4 silent wrong results; rule implementation → warp-f6: step 1 40ac69a3, step 2 506e6b94 (fractional pow via m.pow requested) |
| A6 | stale #[ignore]s: A2/A3 showed ignored tests that already pass — run all ignored tests, un-ignore the passing ones | done c519d7fe..bfcd81bd (10 un-ignored, 657/0/123); A6b: none newly passing; 67 ignored tests still fail (probes/ignored_run.log) |
| A7 | notes/footguns.md plain-bugs list stale (6 of 7 fixed) → pointers to regression tests | done 9a436c80 |
| C2 | src/node.rs:412 `Text("TODO: …")` placeholder → real implementation or loud error | done 768e8a87 (Node::todo has no callers) |

## Low
| id | task | status |
|---|---|---|
| C3 | tests/test_node.rs:7 Node `remove`; tests/test_node_operators.rs `Meta` not exported | Meta: not applicable (old API gone, comment kept); remove: done e88399a6 (verified by warp-numeric) |
| B5 | semicolon: block literals ({},(),[]) are values keeping all items; running a block (root, function body, if/while) yields its last item (wiki/list.md); replaces de18c3bb heuristic. Step 1: survey conflicting tests | user decided: block literals are values, running a block yields last item; test_lists.rs:205 may change → done ff9b2d62 b2f931d1 |
| B6 | bare name of a function with parameters (`sq:=it*2;sq`) → silent 0, should be loud | done 7e2e746c 417f3fed |
| B7 | test_angle.rs test_switch / test_switch_evaluation: object key lookup {a:1 b:2}[a] | done 82bd757f 72c51ed5 |
| A8 | test_math.rs superscript powers 3⁴, vulgar fractions ⅓9 | assigned warp-numeric |
| C6 | test_meta.rs 5 ignored failing tests (Meta API) | test_parent_context done ba38aea1; @ attributes done 74383853 (refactor to parse_symbol requested), IndexMut meta, meta serialization → warp-library |
| B8 | test_lists.rs 6 ignored failing array tests | parked: tests contradict recorded decisions (size in bytes, checked index assignment); typed-array declaration + while value need user |
| B9 | `while cond: body;rest` colon body swallows `;`; compiler panic (unwrap undefined variable) → error value | assigned warp-parser |
| C4 | tests/test_string.rs:143/150 string operator overloads (`"a".s() + 2`) | parked: user doesn't need it, commented lines stay TODO |
| C5 | src/meta.rs:9 conditional compilation | done edd78b86 (LineInfo.line debug-only) |
| L1 | test_web.rs:43 `$b.ok` emitAttributeSetter; :71 Externref kind | parked: needs a real webview host (wry?) + Externref Kind → user |
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

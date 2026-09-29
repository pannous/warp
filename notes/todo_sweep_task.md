# Task: TODO sweep (2026-09-29)

Baseline: ./test.sh → 611 passed, 0 failed, 132 ignored (commit after 0d131407). Issue #2 closed (server file restored).
Three agents work in the SAME checkout on disjoint packages. A reviewer (the spawning session) checks each commit.

## Package A "numeric" — function typing (session warp-numeric)
1. tests/test_law.rs:52 `#[ignore = "next"]`: `:=` functions compile `x:float` params as Int, `half(1.0) == 0`.
2. tests/probe_footguns.rs:129 `#[ignore = "next"]`: `(x*x) as i64` panics inside a function body.
3. tests/test_math.rs:99 and :287 `#[ignore = "soon"]` (mixed-type variables / automatic casting).
4. notes/footguns.md "Plain bugs": `1/4+1/4` → 0 (sum typed Int). Verify first — some entries may already be fixed.

## Package B "parser" — precedence and sequences (session warp-parser)
1. tests/test_functions.rs:139/145/151: `:=` vs newline precedence ("use newline once parser precedence is fixed").
2. tests/test_angle.rs:35: function application without parens precedence.
3. tests/test_string.rs:221: statement sequences `'hello';(1 2 3 4);10` → 10.
4. notes/footguns.md "Plain bugs": `f := it*10; 1 + f 3` → should be 31; `x=[1 2 3]; x[3]` returns program text. Verify first.

## Package C "library" — Rust API TODOs (session warp-library)
1. src/main.rs:208 "don't run, just compile and save binary" — compile-only CLI path writing the .wasm.
2. src/node.rs:412 returns `Text("TODO: …")` as a placeholder — replace with a real implementation or a loud error.
3. tests/test_node.rs:7 Node `remove` method; tests/test_node_operators.rs:3/121-124 `Meta` not exported from `warp::*`.
4. tests/test_string.rs:143/150 string operator overloads (`"a".s() + 2`).
5. src/meta.rs:9 conditional compilation — only if cheap, else leave and report.

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

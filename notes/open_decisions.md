# Open decisions for the user

Only open questions: pending or parked (user 2026-10-08: "It's called open decisions for a reason. Open"). Only the
Interviewer asks the user; nothing here blocks, each question names the default the code follows. Answers move to
notes/decisions.md, work goes on the board (`todo add`), rules go where they are applied (AGENTS.md, notes/agents/,
the wiki).

## Pending
- single-entry (warp-worker via warp-integrator, branch single-entry 014f1f0c7, parked unmerged; asked 2026-10-11, no
  answer yet): is `{a:1}` a one-entry map (recommended: braces always make a map; data_blocks_stay_data:13 and
  a_value_of_a_pair_is_held_as_a_node:9 change meaning, `{a:1} == (a:1)` becomes no) or the pair (today; branch dropped)?

## Parked (user: "later")
- P237 (warp-class, card natural-phrases): what `numbers.sort by size` means; today a silent no-op. The user leans to
  `size` as an alias chain size → abs → norm (`[3, -5, 2].sort by size` → [2 3 -5]). Open: does `size [3, 4]` stay
  the count 2 (recommended, norm via `norm`) or become the norm 5? Parked 2026-10-09.
- P226c: subscript log base `₁₀⌟100` → 2 (recommended yes), next to the decided `100⌟10` → 2. Parked 2026-10-09;
  skip! in test_logarithm2.
- P150 license: none yet, so nobody may legally reuse the code and crates.io refuses the upload (P151). MIT
  (recommended) / Apache-2.0 / MIT OR Apache-2.0. Parked 2026-10-06.
- P69a: may a run-time block assign the `!` site's local variables? Default (wiki/charged.md): no, it reads them as
  they are at `!` and assigns only declared globals. Parked 2026-10-05 until run-time `!` is built.
- P76: grant syntax for run-time blocks (pure by default): `def f(b:block) ! IO` (recommended) / an argument on the
  forcing word `interpret(x, grant: [io])` / a pragma `use eval io`. Parked 2026-10-05: no grants exist yet.

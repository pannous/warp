# Open decisions for the user

Only the Interviewer asks the user (notes/roles.md). Nothing here blocks: each question names the assumption the code
already follows. Answers move to notes/decisions.md (newest Decided section on top) with the date and the user's
words; this file keeps only what is open: pending and parked questions and user to-dos. (user 2026-10-08: "It's called open
decisions for a reason. Open"). Rules belong where they are applied: AGENTS.md, notes/agents/, the wiki. Older references to
"open_decisions.md" Decided sections, P-, D- or #-numbers mean notes/decisions.md.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

## Pending questions (ordered by impact; recommended option first)
(none)
Parked (user: "Later"):
- Parked: P226c subscript log base `₁₀⌟100` → 2 (recommended yes) next to the decided `100⌟10` → 2. User
  2026-10-09: "later". Stays in skip! in test_logarithm2 (warp-class).
- Parked: P150 license: warp (and warp) have none, so package managers list no license and nobody may legally reuse the
  code. MIT (recommended, as uniscript) / Apache-2.0 / MIT OR Apache-2.0 (Rust convention). User 2026-10-06: "let's
  postpone the license". Blocks the crates.io upload of P151 (crates.io refuses a crate without license metadata).
- Parked: P69a may a run-time block assign the `!` site's local variables? Spec default (wiki/charged.md): no, it reads them
  as they are at `!` and assigns only declared globals. User 2026-10-05: "Later"; revisit when run-time `!` is built.
- Parked: P76 grant syntax for run-time blocks (pure by default): `def f(b:block) ! IO` (recommended) / an argument on the
  forcing word `interpret(x, grant: [io])` / a pragma `use eval io`. User 2026-10-05: "Later": no grants exist,
  run-time blocks are always pure. Asked by warp-29.
- Parked: #10 Polish notation for .wat/.wast, user "Keep parked" 2026-10-03.

## User to-dos (not questions)
- Cloud-Microsoft environment setup script needs `rustup target add wasm32-wasip1` (claude.ai/code → chevron next to
  the session title → Edit cloud environment). From BOSS-cheeky-shannon.

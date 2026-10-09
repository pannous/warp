# Open decisions for the user

Only the Interviewer asks the user (notes/roles.md). Nothing here blocks: each question names the assumption the code
already follows. Answers move to notes/decisions.md (newest Decided section on top) with the date and the user's
words; this file keeps only open decisions: pending and parked questions, nothing else (user 2026-10-09: remove what is decided and what is not a decision; user to-dos go on the board). (user 2026-10-08: "It's called open
decisions for a reason. Open"). Rules belong where they are applied: AGENTS.md, notes/agents/, the wiki. Older references to
"open_decisions.md" Decided sections, P-, D- or #-numbers mean notes/decisions.md.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

## Pending questions (ordered by impact; recommended option first)
- P238 (warp-fixer, card generators-function) what a plain call of a generator gives. Today (recommended):
  `count_to(3)` collects the list [1 2 3], and `sum(count_to(4))` is 10. It is an object only when a variable
  holding it is advanced with `next(c)`/`c.next()` or when written `iter(count_to(3))`. A `for` over it always runs
  lazily. Alternative (Python): every call gives a generator object, so `count_to(3)` prints as an object and
  `list(count_to(3))` collects.
- (warp-types, card let-comma) what `let a, b = 3, 4` declares (also `var`, `const`). Today, assumed (recommended,
  Python, the same as the undeclared `a, b = 3, 4`): unpacking, a = 3 and b = 4. Alternative (JS): two declarations,
  a without a value (an error for let/const) and b = 3, the 4 left over.
Parked (user: "Later"):
- Parked: P237 (warp-class, card natural-phrases) what `numbers.sort by size` means; today a silent no-op. The user
  leans to `size` as an alias chain size → abs → norm ("carries over to vectors … that have a norm"; numbers |x|,
  `[3, -5, 2].sort by size` → [2 3 -5]). Open: does `size [3, 4]` stay the count 2 (recommended, norm via `norm`)
  or become the norm 5? User 2026-10-09: "later".
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


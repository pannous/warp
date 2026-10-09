# Open decisions for the user

Only the Interviewer asks the user (notes/roles.md). Nothing here blocks: each question names the assumption the code
already follows. Answers move to notes/decisions.md (newest Decided section on top) with the date and the user's
words; this file keeps only what is open: pending and parked questions and user to-dos. (user 2026-10-08: "It's called open
decisions for a reason. Open"). Rules belong where they are applied: AGENTS.md, notes/agents/, the wiki. Older references to
"open_decisions.md" Decided sections, P-, D- or #-numbers mean notes/decisions.md.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

## Pending questions (ordered by impact; recommended option first)
Held until 9 AM 2026-10-09 (user: "no more questions till 9 AM"):
- P226b (warp-worker, test_logarithm2): ⌟ is a postfix log (`ℯ⌟` → 1, natural log). What do the forms with a base
  give: `100⌟10` (log base 10 → 2?), `10⌟100`, `₁₀⌟100` (C++ wasp had `10⌟` as one token = log10)? Default: `x⌟` is
  ln x, `x⌟b` is log base b of x (→ `100⌟10` = 2); `10⌟100` reads as log base 100 of 10 = 0.5.
- P228 (warp-fixer, card standalone-std-io): should a program using tables or JSON also build as a stand-alone
  executable? It puts the Node reader, SQLite and adapters into the ~1 MB runtime, against notes/aot.md. Options:
  (a) no (default): a native run notes "host.std_io needs runtime." on the first run only; (b) yes, a bigger runtime;
  (c) yes, only the pieces the program uses (linked per program).
- P229 (warp-worker, card g_gHmE): double-clicking a .warp file in Finder: (a) runs `warp <file>` in Terminal
  (default, registered on this Mac); (b) opens it in the text editor, running stays a command; (c) asks Run or Edit.
- P230 (warp-worker, card effects-value): `effects of f` is now a value; should it be a list of texts
  `["State" "IO"]` like f.params (a, default on branch interpolation-passes; one assertion in
  tests/control/test_variable_signals.rs changes from "(State IO)") or symbols `(State IO)` (b, needs symbol values
  in the emitter)?
Parked (user: "Later"):
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

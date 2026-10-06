# Open decisions for the user

Only the Interviewer asks the user (notes/roles.md). Nothing here blocks: each question names the assumption the code
already follows. Answers move to notes/decisions.md (newest Decided section on top) with the date and the user's
words; this file keeps only pending and parked questions, user to-dos and standing rules. Older references to
"open_decisions.md" Decided sections, P-, D- or #-numbers mean notes/decisions.md.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

## Pending questions (ordered by impact; recommended option first)
(the ones below the user answered "Later"/postponed are parked; reviewed 2026-10-06 evening: P70c and P64 decided,
P69a and P76 still wait for run-time `!` and narrowed grants (P88), #10 unchanged)
- Parked: P150 license: warp (and wasp) have none, so package managers list no license and nobody may legally reuse the
  code. MIT (recommended, as uniscript) / Apache-2.0 / MIT OR Apache-2.0 (Rust convention). User 2026-10-06: "let's
  postpone the license". Blocks the crates.io upload of P151 (crates.io refuses a crate without license metadata).
- Parked: P69a may a run-time block assign the `!` site's local variables? Spec default (wiki/charged.md): no, it reads them
  as they are at `!` and assigns only declared globals. User 2026-10-05: "Later"; revisit when run-time `!` is built.
- Parked: P76 grant syntax for run-time blocks (pure by default): `def f(b:block) ! IO` (recommended) / an argument on the
  forcing word `interpret(x, grant: [io])` / a pragma `use eval io`. User 2026-10-05: "Later": no grants exist,
  run-time blocks are always pure. Asked by warp-29.
Parked: P160 `$1` as a reference to the node with id 1 (wiki/reference.md) vs `$0`/`$1` as lambda parameters.
  User 2026-10-06: unsure; finds both numbered parameters and `it` dangerous across contexts. Next step: warp-42
  writes notes/implicit_params.md listing where `$n` and `it` change meaning (nested lambdas, data literals, loops,
  handlers, and $0/$1 as WebAssembly positional arguments, user 2026-10-06) with concrete examples, then the question comes back with options. Until then: `$n` is a reference only
  when an enclosing node of the same literal declares that id (undoable worker default).
Parked: #10 Polish notation for .wat/.wast, user "Keep parked" 2026-10-03.
Dropped as answered (handover 2026-10-06 "Needs the user" list): eval_untrusted limits (P88 follow-up "Everything,
untrusted too"), the AGENTS.md paragraph (P93), stash and obsolete tests (cleanup rule, P96), the git hook "line 240"
fix (superseded: ~/.claude/hooks/git_destructive.py was reworked after that list, 8fc7e6b and a22714f; whoever still
meets a hook bug files it anew with the exact command).
Dropped as answered: code quality 7 (Node operators return Node::Error: Decided #1, errors as values); #14 (test_math
uses near!), #14c (exit(0) commented out), #15 (decided: delete), #17/#18 (done), #20 (AGENTS.md fixed; CLAUDE.md → P12),
#24 (upto decided exclusive 2026-10-02), #29 (checkout is only behind now), D5 detail (notes/matching.md accepted).

## User to-dos (not questions)
- Cloud-Microsoft environment setup script needs `rustup target add wasm32-wasip1` (claude.ai/code → chevron next to
  the session title → Edit cloud environment). From BOSS-cheeky-shannon.

## Standing rules (user)
- Roadmap, next big features, NOT to be started yet (2026-10-06): "a standard library standard functionality in
  different modules a solid module manager and then a packet manager for internal and external packages as well as
  using existing packaging managers and packages to our greatest advantage". Board cards in Later, in this order:
  stdlib-standard, module-manager, package-manager. Nobody takes them until the user releases them.
- Word choices are not questions (2026-10-06): "we have the alias mechanism to generally tell people if they use the
  wrong word what the right word is but still keep the synonym working or replacing". When the alternatives are only
  different words for the same thing, the recommended word is canonical and the others become aliases: they work,
  with a got-it note naming the right word and an "I meant: <word>" fix (normalize::advise, notes/fixits.md;
  silent synonyms: SYNONYMS in src/lowering/library_words.rs). Only real differences in meaning go to the user.
- Taking tickets (2026-10-06): "When picking a new task from the project, can you mark them as having an SNI
  (asignee)? If we don't have SNI's, then just use me." (SNI = assignee.) `todo take <card> <session>` assigns
  pannous, sets the board field Agent to the session and moves the card to Now (80bef43d8).
- Board tickets (2026-10-06): "There should be the rule to only close or move project tickets with a commit linked in
  the description. Enforce that rule texturally and in the to-do helper." Enforced in AGENTS.md,
  notes/agents/common.md (c4db425c1) and ~/dev/bin/todo (no move to Done without a linked commit).
- Picking tasks (2026-10-06): "When picking new tasks, check if there are some fresh ones under 'Next' that are easily
  done".
- Cleanup (2026-10-03): "Don't ask for my confirmation to delete old stuff": merged branches, stale copies, leftover
  stashes (so warp-90's stash goes without a question; where the hook blocks, the user gets the one-line command).
- Test upgrades (2026-10-05, "allow all tests to be upgraded from a dumb thing to a better thing, from not working to
  working"): an error/refusal/"not yet" expectation becomes the working value, ignored tests that pass are
  un-ignored, without asking; a change of meaning (one working value into another) still needs a decision.


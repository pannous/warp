# Standing rules (user)

Rules the user decided that hold for all work, not single decisions (those are in notes/decisions.md).
Open questions are in notes/open_decisions.md, and only there.

- Roadmap, next big features, NOT to be started yet (2026-10-06): "a standard library standard functionality in
  different modules a solid module manager and then a packet manager for internal and external packages as well as
  using existing packaging managers and packages to our greatest advantage". Board cards in Later, in this order:
  stdlib-standard, module-manager, package-manager. Nobody takes them until the user releases them.
  Released 2026-10-07 (to warp-96): the standard library, including adapters to other standard libraries. Work starts
  as soon as the board's Now/Next columns are practically empty. stdlib-standard is in Soon; module-manager and
  package-manager stay in Later. Lead: the functions worker (stdlib-standard is the next big topic, 2026-10-08; decisions P169-P193).
  Design questions come to the Interviewer, each with a default.
- Lax versus strict (2026-10-08, P203): annotations make it strict. Unannotated code is lax (Python-like, run-time
  checks); anything annotated is a promise the compiler enforces; `--strict` warns about the lax spots.
- Explained questions (2026-10-08): "make these small explanations a general rule for all discussions": every
  question and option comes with a small explanation and example (notes/agents/interviewer.md).
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

- Samples (2026-10-08): every major or semi-major feature ships with a sample in samples/, especially the ORM and the
  server (notes/agents/common.md).
- open_decisions.md holds only what is open (2026-10-08): "It's called open decisions for a reason. Open, you get
  it?" Pending and parked questions and user to-dos; answers go to notes/decisions.md, standing rules here.
- notes/decisions.md is grep-only history (2026-10-08): "If it's really grep-only, then it's perfect. Otherwise, move
  them to a history file." Nobody reads it front to back; should a role ever need to read it whole, move the built
  decisions to a history file first.

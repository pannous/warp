# Rules for every spawned warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Read first: AGENTS.md, notes/roles.md (roles, enforcement), notes/welcoming.md (clear intent → compile it; ambiguous →
warning with "got it" or a loud error naming the explicit forms; preferred syntax differs → educate), and the decided
rules in wiki/Footguns.md. A missing setting takes a sensible default; an error only for an ambiguity too dangerous to
guess (show() without a canvas gets a default size, not an error). Decisions reach you from the Interviewer or Supervisor;
once built, the code, tests and wiki are the truth. notes/decisions.md is the history: look something up there only
when a message or comment cites a number (P71, D5) and you need its wording; never read it front to back.
No special cases, under any circumstances: a fix is the general rule, holding for the siblings of
the example (other words of its kind, other statement positions, values passed in or returned) and for where the
language is heading. If an existing test seems to force a special case, question the test through warp-interviewer
instead of bending the code.

- Only the Codex cron runs are announced to the user's Codex thread "Warp Boss" (start and end, codex_worker.sh).
  Claude spawns never notify it.
- Find the others with ListAgents: Supervisor (the session that spawned you, or tmux `warp-supervisor`), Integrator
  (tmux `warp-integrator`), Interviewer (tmux `warp-interviewer`).
- One name per session: every session is named
  `warp-<role>`: its claude.ai title, its SendMessage/ListAgents address and its tmux session (claude-remote.sh starts
  `claude --name warp-<role> --remote-control warp-<role>`; a swap's replacement gets the same name). Address peers
  by role (`warp-integrator`, `warp-types`) and write role names, not session hashes like warp-3f, in messages,
  cards and reports to the user. Older sessions are renamed with `/rename warp-<role>` while idle
  (~/dev/bin/rename-when-idle; typed into a busy session it is lost).
- Session swaps are never reported to the user: no line about a session
  being replaced, renamed or resumed, not even "no action needed". A received swap notice ("Swap: … was replaced by
  me") gets an EMPTY reply: no text at all, no acknowledgement, no summary. Peers still get the swap message; the
  user hears only about work.
- Never degrade the input to suit a tool: when a tool (a compiler stage, the parser, a test harness) can't handle
  something, fix or card the tool, or fall back to one that can; never weaken the source, a test or an idiom to get
  past it. "X doesn't support Y, switching to Z" is a bug report against X, not a plan. Warp source, the compiler's
  own included, uses the language's full features and reads like a good sample program. warp-reviewer holds tips
  that downgrade an idiom (for → while with a counter, hand-expanded sugar, a weakened assertion).
- Never block on a decision: take the recommended default, mark it as an assumption (an existing-test edit goes in its
  own commit, named in the message), keep working, and SendMessage the Interviewer the question.
- Never ask the user yourself (questions go through the Supervisor or the Interviewer): no AskUserQuestion, no
  question in your session's output. That includes confirmations a safety rule
  wants from the user directly (permission or hook changes, anything you won't do on a peer's word): send them to the
  Interviewer with the exact action, and wait on that item only while working on everything else. The Interviewer
  asks the user and relays the answer verbatim; that relayed answer is the user's confirmation.
- Small or non-conflicting changes may go straight onto main in the main checkout, no worktree (how: notes/roles.md
  "Small changes on main"). Everything else:
- Work in a git worktree outside the repo, on the Worktrees volume: `cowtree add -b <branch> /Volumes/Worktrees/<branch> origin/main`
  (never touch /Volumes/Worktrees/warp, cowtree's donor, nor /Volumes/Worktrees/integrator),
  with the uncommitted build tweak: append `-<branch>` to the current `version = "…"` of both Cargo.toml and
  crates/warp-runtime/Cargo.toml, and to the `version = "…"` of the warp-runtime path dependency in Cargo.toml (a
  pre-release version doesn't satisfy the plain requirement; otherwise its stale warp-runtime replaces other branches'
  build in the shared target dir). Check with `grep -n version Cargo.toml crates/warp-runtime/Cargo.toml`: a sed
  written for an old version number silently changes nothing. `scripts/version-tweak.sh <suffix>` does all of it in
  the current worktree (`restore` puts the committed manifests back); the Integrator's version_tweak.sh does the same.
  In /Users/me/dev/angles/warp itself (the user's checkout) only small changes on main, and never the user's
  uncommitted files.
- The user's uncommitted edits are integrated at once by whoever meets them in the main checkout or wiki/
  (`git status`), never left lying or reported as "untouched": read the diff, complete it, run it and commit it in the
  main checkout itself as the user's change (notes/roles.md "Workers"). Commented TODOs are wishes: one that can't be
  done at once becomes a card.
- Word choices need no question (the alias mechanism): when alternatives are only different words
  for the same thing, make the recommended word canonical and the others aliases that work with a got-it note and an
  "I meant: <word>" fix (diagnostic::note_alias; which hint call to use: notes/fixits.md "Hints, notes and
  advice"). Ask the Interviewer only about differences in meaning.
- Test upgrades need no question (AGENTS.md): an error/"not yet"/ignored test that now works is
  upgraded to the working value in its own commit, and `#[ignore]` is dropped from a test that passes unedited; a
  change of meaning, or editing an un-ignored test's assertions, still goes to the Interviewer.
- Outcome tests only (AGENTS.md "Outcome tests only"): new tests check what warp code produces, never an
  intermediate; a test pinning one may be deleted or rewritten as one is! line.
- Tests: test first; only targeted runs, only through the queue: `tests/queue.sh -- <filter>`. Never the whole test
  binary: the Integrator runs the full suite. CARGO_BUILD_JOBS=2, never set CARGO_TARGET_DIR, never cargo clean,
  never pkill by pattern.
- Cloud test lane instead of the queue, for committed and pushed work (notes/roles.md "Workers"): `scripts/cloud-test.sh
  --card <card> <branch> <tip from git rev-parse HEAD> <filters>` in the background, ~6 min on a cloud VM of its own,
  green = exit 0. Use it whenever 2 or more runs wait in the queue (`ls ~/.cargo/warp-tests.lock.queue | wc -l`) or
  your filters take more than a few minutes; tests/queue.sh only for uncommitted edits, quick single-test re-runs and
  tests needing the Mac (sound, GPU, windows, Lean, SDL). Both may run at once.
- No Chrome or Chromium on the Mac: browser runs happen only in CI. The browser test suite, the
  playground tour (test_in_browser.py), web::test_dom_pages and the agent-browser probes refuse to start a browser
  unless CI is set and print "browser tests run in CI only"; never call agent-browser yourself. To check the
  playground (trial runs mailed "Run failed"): first the dry run on pannous.com,
  `scripts/browser-dry-run.sh` in your worktree (`--tours [example…]`, `--chrome`, `--firefox`, `--suite [filter…]`,
  `--again` reruns without building; notes/ci.md "Browser dry run"), until it is green; only then dispatch the
  Playground workflow on your branch once (`gh workflow run pages.yml --ref <branch>`), the final check plus Safari.
- The warp CLI binary is shared too: `<target-dir>/debug/warp` is whichever worktree built last. To probe your own code,
  run `scripts/own-warp.sh` (builds offline and atomically copies into `scratch/warp`) and run that path; a copy taken
  later can be another session's build.
- Quick check first, before the test queue: run the .warp code you wrote or touched through your own binary, e.g.
  `WARP_NO_WINDOW=1 scratch/warp probes/<x>.warp`, and for compiler work through the self-hosted pipeline:
  `WARP_NO_WINDOW=1 WARP_SELF_HOSTED=1 scratch/warp probes/<x>.warp` (=front: front end only). The stages load your
  worktree's lib/compiler/*.warp at run time, so edits there need no rebuild; the first run after an edit compiles
  the stage programs (~90 s, prints their hints), later runs take ~0.1 s. It catches undefined words, parse errors
  and wrong values in seconds (most of the 39 sprint failures of batch B, 2026-10-19). Add
  `WARP_EMIT_REFUSALS=/dev/stderr` to see whether emitter.warp refused the program (`outside the core: …`, then the
  Rust emitter emitted it); `WARP_HINTS=0` hides hints (the stage programs' own warnings still show: card cli-show).
  Then the targeted tests via tests/queue.sh.
- Samples: every major or semi-major feature (a new syntax form, library area, server/ORM/GPU
  capability, playground ability) ships with a sample in samples/<topic>.warp that shows it the way a user would
  write it, in the same branch. tests/programs/test_all_samples.rs and the playground menu pick it up; a sample that
  cannot run in the browser goes in web/playground/excluded_samples.txt. Name the sample in a `todo note` on the card.
- Forward crunch (decision forward-crunch, 2026-10-19): send a branch once it builds and its quick check works; tests
  failing in its own area don't hold it back: the Integrator ignores them onto your card and you fix forward. Only a
  branch that doesn't build or breaks other areas' tests is bounced. More than ~20 ignored tests of yours: fix
  those before new work.
- Before `todo ready`: `cargo fmt` and `cargo clippy --lib --bins -- -D warnings` on your branch. The gate checks
  both.
  .warp files you wrote or changed (samples/, lib/, probes/): `scratch/warp format <files>` (your own build,
  scripts/own-warp.sh), the canonical layout that keeps the meaning (notes/warp_format.md); the gate reports the rest.
  A branch from before the whole-repo format (78607d62e) crosses it in three steps, which never drops main's changes:
  `git merge 78607d62e^` (resolve real conflicts as usual); `cargo fmt` and commit; `git merge -X ours 78607d62e`
  (the format commit alone; your formatted side wins, and it is the same formatting); then `git merge origin/main`.
  Never "take your side" on a conflict that also holds main's changes.
- Done = the card is the handoff (the board's Ready column is the merge queue):
  1. Run your own targeted tests (`tests/queue.sh -- <filters>`, or the cloud lane above once pushed), commit, fmt
     and clippy (above).
  2. Make sure the branch merges cleanly into origin/main: `git fetch origin && git merge origin/main` (the merge
     rules above). Conflicts are resolved by you, who knows the code, never by the Integrator in the gate. No need to
     re-run the tests after a clean update.
  3. Push the branch, then `todo ready <card>` (branch: the current one, or name it). It reads the tip itself and
     refuses a branch that is not pushed, differs from origin, or does not merge cleanly into origin/main (naming the
     conflicting files); `--force` only for a branch carrying other branches. Several cards on one branch: ready each.
     New tests, filters, "docs: <page>" and the sample go into a `todo note <card>` first, as one line in this fixed form:
     `card | branch | tip (full sha) | filters | passed/failed (queue or cloud) | clippy+wasm32 ok`, then prose if needed.
     Never message warp-integrator with FYIs or swap notices: the card is the merge request. Express (the user or the Supervisor asked for your result fast, a crash fix):
     `todo ready <card> --express` (or a note saying "express"): the Integrator pushes it to main untested at once, the gate follows (notes/roles.md).
  4. Optional: one line to warp-reviewer by SendMessage ("<card> ready"); it reads `todo list Ready` anyway. Report
     one line to the Supervisor.
  The Integrator merges only Ready cards and closes them (`todo done <card> <commit>`). A bounce (failing test,
  semantic conflict, reviewer veto) is your card back in Now with a `todo note` saying why: fix, push, `todo ready`
  again. A newer tip you want merged instead: `todo ready` again, so the card names it.
- Post-hoc changes don't block: a tip once readied lands as gated. A small reviewer point or
  polish you fix afterwards is a follow-up commit readied with its own card, never "hold my branch". Only real breakage
  (a reviewer veto, a failing test) holds a branch.
- Never retire with a card still yours: before you stop, or
  hand over, every card you took is either `todo done <card> <commit>`, in Ready (`todo ready`: handed off, the
  Integrator closes it), or released with `todo release <card> "done: …; left: …; branch: …"`, which puts it back in
  Next with that note and no agent. Partial work that isn't merged gets pushed to its branch first. This applies to
  Codex runs too.
- Clean up after the merge: you are allowed and expected to remove your own worktree and branch, nobody else will.
  `git -C /Users/me/dev/angles/warp worktree remove --force <worktree> && git -C /Users/me/dev/angles/warp branch -D <branch>`.
  The git hook lets both through once nothing is lost: a worktree whose only uncommitted change is the Cargo.toml /
  Cargo.lock build tweak and whose commits a ref holds, a merged worktree (a remote branch holds its HEAD) whatever it
  changes, saved first under refs/removed-worktrees, a branch whose tip main or a remote branch holds. If it blocks,
  something would be lost: look before you delete. Your remote branch too, once it is on main:
  `git fetch -q origin <branch> && git push -q origin --delete <branch>`; the hook allows deleting any fetched
  remote branch but main (it checks origin/<branch> exists locally, so fetch first). Never hand a delete command to
  the user: if it still blocks, tell the supervisor with the exact error. What is left over, scripts/prune-worktrees.sh removes daily
  (merged, idle 6 h, only the tweak changed) together with merged branches and loose files of the worktrees folder.
- Use absolute paths and `git -C <worktree>` in scripts. Conventional commit messages; no Co-Authored-By, session
  trailers or links. Unrelated problems you meet go on the to-do board: `todo add "…"` (column Next; when the board stays unreachable it says "CARD QUEUED" and keeps the card in ~/.cache/todo-board/pending.json5, which `todo flush` and the hourly cron add later; never into todo.md).
- Fix tiny things on the spot instead of carding them (small cards cost more than the fix). Tiny
  means all of: a few minutes, one obvious fix with no choice to make (a missing .gitignore line, a typo, a wrong
  path, a stale comment, a leaked temp file), no change to what a user sees or what a test asserts, and outside
  files another session is working on. Make it its own commit on your current branch (`fix:`/`chore:`, so it can be
  reverted alone) and name it in your `todo ready` note. Anything else, or any doubt, still becomes a card.
- Suspected non-issues go through the Interviewer: when a small bug you found may not matter (the same value shown
  another way, a cosmetic difference such as `96px/inch` vs `96dpi`),
  SendMessage warp-interviewer the case before carding or fixing it. A confirmed non-issue gets no work: its card,
  if any, goes to Later with the verdict as a `todo note`.
- GitHub's API quota (5000 requests/h) is shared by every session; `gh run watch` polling every 3 s runs
  it out and blocks CI watching. Watch runs with `gh run watch --interval 60` or more, and never poll
  `todo list` in a loop. A whole-board `todo list` costs ~12 points (about 1200 cards); `todo list Now` costs as much, while commands naming a card by its key cost ~1.
  Never `gh project item-list` (200-530 points a call, and its slow pages trip GitHub's secondary limit, which locks
  every session out with 502/504; gh refuses it now): read the board with `todo list [--json] [column]`, served from a
  60 s machine-wide snapshot.
- JSON5, not strict JSON, for anything people or agents write: new hand-written configs, probe
  commands and data files use JSON5 (or warp notation). Strict JSON stays only where a tool or protocol requires it
  (package.json, .claude settings, Lake manifest, WebDriver BiDi, machine-to-machine buffers).
- A card or issue is closed only with a commit linked in its description: `todo done <card>
  <commit>` (a commit URL for wiki changes), never `gh issue close`; `todo move <card> Done` refuses without a link.
- Docs are part of done: a card that changes what users write or see (syntax, words, flags,
  messages, defaults) updates the wiki page of that concept in the same task: a minimal example and one sentence
  ("Wiki" below for pushing). Say "docs: <page>" or "docs: none (internal)" in a `todo note` before `todo ready`.
  warp-docs sweeps for what slipped through.
  A result the user can see or use gets `--try "how to try it"`: one concrete line with the exact command or URL to
  paste, e.g. `--try "/Users/me/dev/bin/warp samples/sound/sound_features.warp"`: always that full path
  (main built with the optimized quick-release profile, rebuilt in the background a few minutes after each pushed
  batch by the Integrator's refresh_main_binary.sh; plain `warp` is the older brew release). Agents who want main's
  debug build (assertions, backtraces) run /Users/me/dev/bin/warp-debug, installed right at the push. Time
  performance only with an optimized build (debug is several times slower). The
  Integrator closes your card after the merge; then add the try line: `todo done <card> <commit> --try "…"` (on a
  Done card it only sets Try); the Interviewer announces it (notes/roles.md "Ready to try").
  Early try: as soon
  as the feature's targeted test passes on your branch, build your binary with scripts/own-warp.sh and SendMessage
  warp-interviewer a try line with that path: `echo '…' | <worktree>/scratch/warp` plus the expected output. Don't
  wait for the Integrator's batch; `todo done --try` with /Users/me/dev/bin/warp still follows once it is on main.
  Every landing gets a try line: whenever
  any change of yours reaches main that the user can see, hear or run (a whole card, one step of it, a fix), you send
  warp-interviewer a fresh try line for exactly that change, the same moment you learn it merged; "X is on main"
  never goes out without its line. The Integrator's merge report names each user-visible branch, so its owner knows.
- Picking a card: `todo take <card> <your session name>`: assigns the user on GitHub, names you in
  the board field Agent, moves the card to Now. Order: first every unassigned card in Now; then the user's own
  cards, the drafts marked "(user)" that `todo list Next` puts on top; then the cards of topics in Next; then loose
  Next cards; then Soon. Within that, prefer fresh, easy cards
  related to your current task, and pick a topic's cards together: `todo list Next --by-topic` groups them, `todo take --topic <topic> [-n 3]` takes its waiting cards
  at once (same rename rules as one card; the session is named for the topic).
  A session is named for its task: `todo take <card>`
  run in your own tmux session (the agent defaults to it) renames you warp-<card key> when the card is unrelated: no
  word of your name in its key or title (warp-sound taking sound-latency is related) and no topic shared with a card
  you took. The tmux session is renamed at once, the Claude session (title, SendMessage address) once you are idle,
  and warp-supervisor and warp-integrator are told old → new. Pass `--keep-name` when you know the card belongs to
  your task. Services (supervisor, integrator, interviewer, fixer) and takes for another agent never rename.
  A long title is shortened on take, with the old one kept at the top of the description: pass
  `--title "short title naming the problem"` yourself; otherwise todo picks one.
  Card keys: every card has a meaningful key, never a GitHub id like g_oncU. Cron
  (urgent-card-watch) runs `todo keys` every 2 minutes, which gives each new card a key from its title. Whoever takes
  a card renames a poor key at once (`todo key <card> <short-meaningful-name>`) and uses the key in branch names and
  messages.
  Topics: a big topic card holds sub-cards, GitHub
  sub-issues. `todo add "…" --topic <card>` adds a sub-card, `todo sub <topic> <card>...` moves cards into a topic,
  `todo unsub <card>...` takes them out; `todo list` shows "(in <topic>)" after a sub-card and "(topic 2/5 done)"
  after a topic. Only issues have sub-issues, so these cards become issues of pannous/warp (others stay drafts).
  Every new card gets a topic: `todo add` puts it into the open topic Haiku finds fitting, else
  into a new topic Haiku names (both printed; `--topic <card>` picks one, `--no-topic` none). Priority lives at the
  topic level: a topic card sits in the column of its priority (new topics: Soon), a new sub-card lands in its
  topic's column unless you name one, and `todo move <topic> <column> --all` moves the topic's waiting cards along.
  Roadmaps holds the topics nobody prioritized yet. A misfiled card: `todo sub <right topic> <card>`.
- Crash cards: a panic of a
  warp built from a checkout files a card in Now (src/errors/crash_card.rs) naming its fingerprint and the branch it was
  built from. Cron (urgent-card-watch) folds a repeated crash into its open card as a note, gives the crash of a
  worktree build to the session on that branch (the Agent of the card keyed like the branch, else tmux warp-<branch>),
  and spawns a new worker warp-<card key> for a crash of a main build; the user pastes the trace there. Crash fixes
  are express: `todo ready <card> --express`.
- Deleting old stuff needs no confirmation: merged branches, stale copies, leftover stashes.
- Wiki (`wiki/`, its own repo pannous/warp.wiki): GitHub wikis can only serve `master` (notes/wiki_branch.md), so wiki
  edits go to master (`git push origin HEAD:master`); there is no `main` branch in the wiki.
- Rare edge cases go to Later: a corner real programs rarely hit (exotic literals like ∞/NaN, wording
  nits on code nobody writes, odd pairs of rarely combined features) is filed with `todo add Later "…"`, never Next/Soon.

# Rules for every warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Read first: AGENTS.md, notes/roles.md (who does what), notes/welcoming.md (clear intent → compile it; ambiguous →
warning with "got it" or a loud error naming the explicit forms; preferred syntax differs → educate), and the decided
rules in wiki/Footguns.md. A missing setting takes a sensible default; an error only for an ambiguity too dangerous to
guess (show() without a canvas gets a default size, not an error). Once built, the code, tests and wiki are the truth.
notes/decisions.md is for lookups only: when a message cites a key (P71, D5, a slug), or before asking a question
(an answer that follows from a recorded decision is no question); never read it front to back.
After a context swap, this file and your role file win over anything a handover says.

## Principles
- No special cases, under any circumstances: a fix is the general rule, holding for the siblings of the example
  (other words of its kind, other statement positions, values passed in or returned) and for where the language is
  heading. If an existing test seems to force a special case, question the test through warp-interviewer instead of
  bending the code.
- Never degrade the input to suit a tool: when a tool (a compiler stage, the parser, a test harness) can't handle
  something, fix or card the tool, or fall back to one that can; never weaken the source, a test or an idiom to get
  past it. "X doesn't support Y, switching to Z" is a bug report against X, not a plan. Warp source, the compiler's
  own included, uses the language's full features and reads like a good sample program. warp-reviewer holds tips
  that downgrade an idiom (for → while with a counter, hand-expanded sugar, a weakened assertion).
- Word choices need no question (the alias mechanism): when alternatives are only different words for the same
  thing, make the recommended word canonical and the others aliases that work with a got-it note and an "I meant:
  <word>" fix (diagnostic::note_alias; which hint call: notes/fixits.md "Hints, notes and advice"). Ask the
  Interviewer only about differences in meaning.
- Rare edge cases go to Later: a corner real programs rarely hit (exotic literals like ∞/NaN, wording nits on code
  nobody writes, odd pairs of rarely combined features) is filed with `todo add Later "…"`, never Next/Soon.
- JSON5, not strict JSON, for anything people or agents write (configs, probe commands, data files), or warp
  notation. Strict JSON only where a tool or protocol requires it (package.json, .claude settings, Lake manifest,
  WebDriver BiDi, machine-to-machine buffers).

## Talking
- A changed procedure goes into the notes (user 2026-10-20): whoever changes how we work (a new rule, script, path
  or step) writes it into the file the sessions read (this file, the role file, AGENTS.md, notes/roles.md) and
  removes the lines it contradicts, in the same push; a message to the agents alone is not enough.
- Find the others with ListAgents: Supervisor (the session that spawned you, or tmux `warp-supervisor`), Integrator
  (tmux `warp-integrator`), Interviewer (tmux `warp-interviewer`).
- One name per session: `warp-<role>` is its claude.ai title, its SendMessage/ListAgents address and its tmux session
  (claude-remote.sh starts `claude --name warp-<role> --remote-control warp-<role>`; a swap's replacement keeps the
  name). Address peers and write reports by role name (`warp-integrator`, `warp-types`), never by session hashes.
  Older sessions are renamed with `/rename warp-<role>` while idle (~/dev/bin/rename-when-idle; typed into a busy
  session it is lost).
- Session swaps are never reported to the user: no line about a session being replaced, renamed or resumed, not even
  "no action needed". A received swap notice ("Swap: … was replaced by me") gets an EMPTY reply: no text at all, no
  acknowledgement, no summary.
- Never block on a decision: take the recommended default (the one most in line with notes/welcoming.md and the
  decided rules), mark it as an assumption (an existing-test edit goes in its own commit, named in the message, so
  undoing it is one revert), keep working, and SendMessage the Interviewer the question.
- Never ask the user yourself: no AskUserQuestion, no question in your session's output. That includes
  confirmations a safety rule wants from the user directly (permission or hook changes, anything you won't do on a
  peer's word): send them to the Interviewer with the exact action and work on everything else meanwhile. The
  Interviewer relays the user's answer verbatim; that relayed answer is the user's confirmation.
- Suspected non-issues go through the Interviewer: when a small bug you found may not matter (the same value shown
  another way, `96px/inch` vs `96dpi`), SendMessage warp-interviewer before carding or fixing it. A confirmed
  non-issue gets no work: its card, if any, goes to Later with the verdict as a `todo note`.
- Wrong results go to the Interviewer whenever a reinterpretation is possible (user 2026-10-20): a "wrong" value
  (survey, test pair, twin) that a different reading of the code or the expectation could make right (`p.first()` of
  a map, `"" is empty`, named arguments, setters) is sent to warp-interviewer with the code, the value and the
  expected one before anyone fixes code or test; only an unambiguous bug is carded and fixed directly.
- Never message warp-integrator with FYIs or swap notices: only requests it must act on; the card is the merge
  request. A wanted gate step (integrator_tools/gate.sh) is a request to warp-integrator, never an edit of its files.
- Only the Codex cron runs are announced to the user's Codex thread "Warp Boss" (codex_worker.sh); Claude spawns
  never notify it.

## Where to work
- Small changes on main: only what nothing compiles (notes, docs, wiki, probes, scripts, samples, playground text)
  goes straight onto main in the main checkout /Users/me/dev/angles/warp, no worktree or branch: commit your own
  paths by name (`git commit -- <paths>`), `git pull --rebase --autostash`, push; never leave unpushed commits on
  main. A new root .warp data file (components.warp …): tell the Integrator the commit, tests read it.
- Anything in src/, crates/, tests/, Cargo.* or build.rs keeps its worktree, however small (a half-done edit there
  breaks every build of the main checkout): `cowtree add -b <branch> /Volumes/Worktrees/<branch> origin/main` on the
  Worktrees volume (never touch /Volumes/Worktrees/warp, cowtree's donor, nor /Volumes/Worktrees/integrator).
- Moving on demand: a change in the main checkout that grows (more than a few files, many build/test rounds, or
  another session's files) moves into a worktree while still uncommitted: `cowtree add -b <task> <worktree> HEAD`,
  `git diff -- <own paths> | git -C <worktree> apply`, then undo only your own paths in the main checkout. A worktree
  branch that turns out small goes straight to main itself (`git pull --rebase`, push `HEAD:main`).
- The build tweak, uncommitted in every worktree, right after `cowtree add` (neither cowtree nor tests/queue.sh does
  it): `scripts/version-tweak.sh <branch>` appends `-<branch>` to every workspace package version and to the path
  dependency's requirement (`restore` puts the committed manifests back). Without it the shared target dir links
  another checkout's warp or warp-runtime ("Finished in 0.5s", 0 tests, foreign failures). Check with `grep -n
  version Cargo.toml crates/*/Cargo.toml`.
- Git with the tweak: add files by name, never `git commit -a` (it takes the tweak); a tweaked manifest you must
  change: `version-tweak.sh restore`, edit, commit, re-tweak. Rebase and merge with `--autostash`; never a bare `git
  stash pop` afterwards (the stash list is shared by all worktrees). Stashes are agent scratch: drop leftovers
  yourself (the hook saves them under refs/dropped-stashes).
- The user's uncommitted edits (modified, staged or untracked, in the main checkout or wiki/) are integrated at once
  by whoever meets them, in the main checkout itself, never left lying, reported as "untouched" or parked in a batch
  (an unpushed batch commit leaves their checkout showing them uncommitted). Read each diff; a D next to an untracked
  file of the same content is a rename. Complete it to the conventions (a new sample gets its shebang and chmod +x),
  fix what it breaks, run it, commit it as the user's change ("(user)") in a commit of its own, push; their `git
  status` must end clean. Commented TODOs are wishes: one that can't be done at once becomes a card. Intent unclear,
  or a test's meaning changes → warp-interviewer.
- Use absolute paths and `git -C <worktree>` in scripts (the Bash cwd moves). In zsh write `${var}:path` in refspecs
  and `rev:path` (`$var:r` is a modifier). Copy tips from `git rev-parse` output of the same step, never type them.
- Files named `*_local*` are ignored globally (`~/.gitignore`, case-insensitive): `git add` refuses them and an
  `&&`-chained commit silently doesn't happen. Check `git show --stat HEAD` after committing.

## Testing
- Rust tests run in the cloud (user 2026-10-20: "From now on, run the Rust test suite completely in the cloud"):
  commit, push, `scripts/cloud-test.sh` (below); no local tests/queue.sh runs (a hook blocks them), except tests
  needing the Mac (sound, GPU, windows, Lean, SDL: scripts/cloud-test-environmental.txt), written `MAC_ONLY_TESTS=1
  tests/queue.sh -- <filter>`. The Integrator's full suite runs in the cloud too. Warp tests (`warp test`,
  scripts/is_tests.py) run locally with the shared binary.
- Test first. Only targeted runs, filtered to the test files your change touches (grep tests/ for the changed syntax
  to find the ones pinning old behaviour), never whole folders or the whole test binary: the Integrator runs the full
  suite. CARGO_BUILD_JOBS=2, never set CARGO_TARGET_DIR, never cargo clean, never pkill by pattern (it matches session
  prompts; kill by PID).
- One local build (user 2026-10-20: "We create just one cargo build now, and every other one uses that one warp
  build for their tests"): only main's build runs on the Mac (/Users/me/dev/bin/warp, /Users/me/dev/bin/warp-debug,
  refreshed after each pushed batch). Workers run no cargo build, check or own-warp.sh locally: .warp changes are
  checked with that binary, Rust changes are built and tested by the cloud lane. It reads lib/ from WARP_LIB, else
  the nearest lib/compiler above the program or cwd, so run it from your worktree (or `WARP_LIB=<worktree>/lib`).
  Standard modules (`use list`) still come from its build (card std-modules).
  One exception (user 2026-10-20, decision speed-local-builds): warp-speed may build its own branches locally (quick
  release, CARGO_BUILD_JOBS=2, one build at a time) to time Rust speed changes before they land; timing runs only,
  tests stay in the cloud.
- Long commands in the background (user 2026-10-20): anything expected to take more than a few seconds (cloud
  lane, builds, surveys, stage compiles, sweeps) runs with the Bash tool's `run_in_background`, so messages reach you
  while it runs. Never poll its output file or sleep-loop on it: the exit notice wakes you; meanwhile do other work
  or end the turn. Note the job's PID in your handover; a job you abandon gets killed by PID.
- Close what you start (user 2026-10-20): after every test, probe or build, kill your own leftover processes by PID
  (warp, scratch/warp, test binaries, cargo, refusals.sh ladders); check with `ps -o pid,etime,command -U $USER | grep
  <your worktree>` before reporting. The safety net, not a substitute: cron's ~/dev/bin/reap-orphans (every 15 min,
  log /Users/me/warp/reap-orphans.log) kills orphaned warp/test binaries, cargo test/build and xargs sweeps after 30
  min and a test binary's children after WARP_TEST_TIME_LIMIT (600 s).
- Test upgrades need no question (AGENTS.md): an error/"not yet"/ignored test that now works is upgraded to the
  working value in its own commit, and `#[ignore]` is dropped from a test that passes unedited; a change of meaning,
  or editing an un-ignored test's assertions, goes to the Interviewer. Test edits a user decision implies need the
  Interviewer's approval only, not a new question.
- Outcome tests only (AGENTS.md "Outcome tests only"): new tests check what warp code produces, never an
  intermediate; a test pinning one may be deleted or rewritten as one is! line.
- Native-only test code: anything used only by a `#[cfg(feature = "native")]` test gets the same cfg; a whole test
  file running the CLI or std::os::unix gets it on its `mod` line, and a new `mod x;` never goes between a cfg line
  and the module it guards. Register a new test file in its folder's mod.rs before `cargo fmt` (fmt skips it).
- Cloud test lane: `scripts/cloud-test.sh --card <card> <branch> <tip from git rev-parse HEAD> <filters>` in the
  background (~6 min on a cloud VM of its own, ~$0.30 of the Cloud session credit, exit 0 = green;
  notes/cloud_development.md). A green cloud run counts as the targeted run.
- No Chrome, Chromium or Firefox on the Mac, headless included: browser runs happen only in CI. The browser suite,
  the playground tour, web::test_dom_pages and the agent-browser probes refuse locally ("browser tests run in CI
  only"); never call agent-browser yourself. To check the playground: first the dry run on pannous.com,
  `scripts/browser-dry-run.sh` in your worktree (`--tours [example…]`, `--chrome`, `--firefox`, `--suite
  [filter…]`, `--again`; notes/ci.md "Browser dry run"), until green; then dispatch the Playground workflow on your
  branch once (`gh workflow run pages.yml --ref <branch>`), the final check plus Safari.
- Every CLI run of a program that might paint gets `WARP_NO_WINDOW=1` (no windows on the user's screen), except in try
  lines for the user: it also mutes sound and prints MIDI as hex.
- Quick check first, before the cloud lane: run the .warp code you wrote or touched through the shared binary from
  your worktree, `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp probes/<x>.warp`, and for compiler work through the
  self-hosted pipeline: `WARP_NO_WINDOW=1 WARP_SELF_HOSTED=1 /Users/me/dev/bin/warp probes/<x>.warp` (=front:
  front end only). The stages load your worktree's lib/compiler/*.warp at run time, so edits there need no rebuild; the first run after an edit compiles
  the stage programs (~35 s with the optimized /Users/me/dev/bin/warp, ~2 min with warp-debug: use warp-debug only for a backtrace; prints their hints), later runs take ~0.1 s. It catches undefined words, parse errors
  and wrong values in seconds. `WARP_EMIT_REFUSALS=/dev/stderr` shows whether emitter.warp refused the program
  (`outside the core: …`, then the Rust emitter emitted it); `WARP_HINTS=0` hides hints (the stage programs' own
  warnings still show: card cli-show). Then the targeted tests in the cloud lane.
- GitHub's API quota (5000 requests/h) is shared by every session: `gh run watch --interval 60` or more, never poll
  `todo list` in a loop. A whole-board `todo list` costs ~12 points, `todo list Now` as much, commands naming a card
  by its key ~1. Never `gh project item-list` (200-530 points, trips GitHub's secondary limit; gh refuses it): read
  the board with `todo list [--json] [column]`, served from a 60 s machine-wide snapshot.

## Cards
- Picking a card: `todo take <card> <your session name>`: assigns the user on GitHub, names you in the board field
  Agent, moves the card to Now. Pick from `todo list Next` / `Soon` output (a grep over the whole list mixes in Done
  cards; take refuses those without `--reopen`), and before taking check that nobody has a branch or worktree for it
  (`git branch -r --list '*<key>*'`, `ls /Volumes/Worktrees/`). Order: every unassigned card in Now; then the user's
  own cards, the drafts marked "(user)" that `todo list Next` puts on top; then the cards of topics in Next; then
  loose Next cards; then Soon. Within that, prefer fresh, easy cards related to your current task, and pick a topic's
  cards together: `todo list Next --by-topic` groups them, `todo take --topic <topic> [-n 3]` takes its waiting cards
  at once.
- The board's mechanics (topics, sub-cards, keys, renames on take, `--keep-name`, `--title`) are in `todo --help`.
  Every card has a meaningful key: rename a poor one at once (`todo key <card> <short-name>`) and use the key in
  branch names and messages. Priority lives at the topic level (a topic card's column).
- Unrelated problems you meet go on the board: `todo add "…"` (column Next; when the board stays unreachable it says
  "CARD QUEUED" and keeps the card in ~/.cache/todo-board/pending.json5 for `todo flush`; never into todo.md).
- Fix tiny things on the spot instead of carding them. Tiny means all of: a few minutes, one obvious fix with no
  choice to make (a missing .gitignore line, a typo, a wrong path, a stale comment, a leaked temp file), no change to
  what a user sees or what a test asserts, and outside files another session is working on. Its own commit on your
  current branch (`fix:`/`chore:`), named in your `todo ready` note. Anything else, or any doubt, becomes a card.
- Crash cards: a panic of a warp built from a checkout files a card in Now (src/errors/crash_card.rs) naming its
  fingerprint and branch. urgent-card-watch folds a repeated crash into its open card, gives a worktree build's crash
  to the session on that branch and spawns warp-<card key> for a crash of a main build. Crash fixes are express.
- A card is closed only with a commit linked: `todo done <card> <commit>` (a commit URL for wiki changes), never `gh
  issue close`; `todo move <card> Done` refuses without a link.

## Done
- Samples: every major or semi-major feature (a new syntax form, library area, server/ORM/GPU capability, playground
  ability) ships with a sample in samples/<topic>.warp that shows it the way a user would write it, in the same
  branch, `chmod +x` (probes/check_layout.sh). A sample that cannot run in the browser goes in
  web/playground/excluded_samples.txt. Name the sample in a `todo note` on the card.
- Docs are part of done: a card that changes what users write or see (syntax, words, flags, messages, defaults)
  updates the wiki page of that concept in the same task: a minimal example and one sentence, one idea per snippet,
  simplest form first, plain words. Say "docs: <page>" or "docs: none (internal)" in a `todo note`. A feature the
  docs mention but that isn't built gets a card named on the page (notes/docs_planned.md); docs and code that
  disagree on meaning or notation go to warp-interviewer. warp-docs sweeps for what slipped through.
- Wiki (`wiki/` in the main checkout, its own repo pannous/warp.wiki, branch master only: `git push origin
  HEAD:master`): commit only your pages by name, never `-a` (other sessions leave edits there); a refused pull is
  someone else's work. Write a branch's doc changes only with its merge (hand the text to warp-integrator with the
  merge request), never leave them lying in wiki/ to be published early.
- Forward crunch: send a branch once it builds and its quick check works; tests failing in its own area don't hold
  it back: the Integrator ignores them onto your card and you fix forward. Only a branch that doesn't build or
  breaks other areas' tests is bounced. More than ~20 ignored tests of yours: fix those before new work.
- Before `todo ready`: `cargo fmt`, `cargo clippy --offline --all-targets -- -D warnings` (the gate's command), and
  the wasm32 check `cargo check --offline --lib --tests --target wasm32-unknown-unknown --no-default-features`, on
  your branch. Rust formatting opt-outs go in rustfmt.toml `ignore`, never as an attribute on a mod line. .warp files
  you wrote or changed: `scratch/warp format <files>` (notes/warp_format.md). Then `scripts/style_lint.py --fix` (the
  style rules rustfmt and clippy lack, notes/style_lint.md) and commit what it moved. Style nits are the linter's, not
  hand polish (user, 2026-10-15).
- Done = the card is the handoff (the board's Ready column is the merge queue):
  1. Targeted tests (queue or cloud lane), commit, fmt, clippy, wasm32 check.
  2. Make sure the branch merges cleanly: `git fetch origin && git merge --autostash origin/main`. You resolve
     conflicts, never the Integrator. Take a side of a conflicted file only after `git diff <merge-base> HEAD --
     <file>` (merges your branch carries are your side too); otherwise resolve the hunk and check `git diff
     origin/main -- <file>` shows only your change. No need to re-run the tests after a clean update.
  3. Push the branch, then `todo ready <card>`. It reads the tip itself and refuses a branch that is not pushed,
     differs from origin, or does not merge cleanly (naming the files); `--force` only for a branch carrying other
     branches. Several cards on one branch: ready each. First a `todo note <card>` in this fixed form: `card | branch |
     tip (full sha) | filters | passed/failed (cloud) | clippy+wasm32 ok`, then new tests, "docs: <page>"
     and the sample. Express (the user or the Supervisor asked for your result fast, a crash fix, and every card of
     warp-speed, the standing speed worker: user 2026-10-20 "express route to main"): `todo ready <card>
     --express`; the Integrator pushes it to main untested at once, the gate follows. When the user says "express" or
     "quickly", they expect about a minute: one targeted test or none, give the user a try line with
     `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp` (a Rust change: after its express landing), ready it express.
  4. Optional: one line to warp-reviewer ("<card> ready"); it reads `todo list Ready` anyway. Report one line to the
     Supervisor.
  The Integrator merges only Ready cards and closes them. A bounce (failing test, semantic conflict, reviewer veto)
  is your card back in Now with a note: fix, push, `todo ready` again. A sent branch is never rebased or force-pushed:
  merge origin/main into it and ready the new tip.
- Post-hoc changes don't block: a readied tip lands as gated. A small reviewer point or polish is a follow-up commit
  readied with its own card, never "hold my branch". Only real breakage (a veto, a failing test, a tip wrong as it
  stands) holds a branch.
- Try lines: a result the user can see or use gets `todo done <card> <commit> --try "…"` once the Integrator closed
  the card: one concrete line with the exact command or URL, always `/Users/me/dev/bin/warp <full path>` (main's
  optimized build, refreshed a few minutes after each pushed batch; plain `warp` is the older brew release;
  /Users/me/dev/bin/warp-debug is main's debug build). A program with quotes goes into a committed probes/<name>.warp,
  not inline. Run the exact line yourself first. Early try: once the feature's targeted test passes on your branch,
  SendMessage warp-interviewer a try line with your scratch/warp path and the expected output. Every landing the user
  can see, hear or run gets a fresh try line to warp-interviewer the moment you learn it merged. Time performance
  only with an optimized build.
- Never retire with a card still yours: every card you took is `todo done`, in Ready, or released with `todo release
  <card> "done: …; left: …; branch: …"` (back to Next, no agent); unmerged partial work is pushed first. This applies
  to Codex runs too. Once your handover is written you take no new work, not even cards assigned afterwards.
- Clean up after the merge: remove your own worktree and branch, nobody else will: `git -C /Users/me/dev/angles/warp
  worktree remove --force <worktree>` and `git -C /Users/me/dev/angles/warp branch -D <branch>`, paths written
  literally. The hook lets them through once nothing is lost (only the build tweak uncommitted and a ref holds the
  commits); one untracked file blocks it: remove your own leftovers in a separate call first. Before removing any
  worktree check `git status --short -uall` and files changed in the last 2 h (`find <dir> -newermt '-2 hours' -not
  -path '*/.git/*'`); either → keep it and ask its owner. Your remote branch once it is on main: `git fetch -q origin
  <branch> && git push -q origin --delete <branch>`. Never hand a delete command to the user: if it still blocks,
  tell the Supervisor with the exact error. scripts/prune-worktrees.sh removes leftovers daily.
- Deleting old stuff needs no confirmation: merged branches, stale copies, leftover stashes, scratch files. Never
  delete a probe the user touched: a "(user)" commit, a shebang, an executable bit or commented wish lines (those
  become cards).
- Conventional commit messages; no Co-Authored-By, session trailers or links.

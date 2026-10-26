# Handbook: the detailed procedures behind common.md

Look things up here when a task needs them; never read it front to back at startup. common.md wins where they differ.
Principles and Talking live only in common.md.

## Where to work
- Quota wind-down : every Claude worker
  finishes and lands the card it is on, writes its handover into the card (`todo release`), then retires. No new
  cards. Only warp-integrator and warp-layers keep running. The sugar passes outside parser.warp belong to the user's
  Grok Bot coders (instructions: scratch/grok-cards.md; they report in scratch/grok-reports.md); warp-layers keeps
  parser.warp only.
- Beautiful types (decision beautiful-types): every lib/ declaration a migration or cleanup branch touches leaves it
  fully and precisely typed. A type that doesn't exist yet is asked of warp-types-back (card beautiful-types), which
  adds the shallow subclasses; don't invent parallel ones in your branch.
- The sugar migration  keeps one line of history: warp-layers alone removes sugar
  from lib/compiler/parser.warp and moves passes into lib/sugar/, one module group per branch, branched from
  origin/main after the engine feature it needs has landed (never stacked on warp-rewrite-engine's branch), landed
  express the same day. Other branches touching parser.warp or a pass on the migration list stay small and land
  express; one older than a day merges origin/main or is closed. Folds and fusion (min_max.warp, broadcasting,
  lambdas.warp's fusion) are warp-folds' until its cards land; migration 3 skips them. Aliases are warp-signatures'
  lib/aliases.warp.
- Small changes on main: only what nothing compiles (notes, docs, wiki, probes, scripts, samples, playground text)
  goes straight onto main in the main checkout /Users/me/dev/angles/warp, no worktree or branch: commit your own
  paths by name (`git commit -- <paths>`), `git pull --rebase --autostash`, push; never leave unpushed commits on
  main. A new root .warp data file (components.warp …): tell the Integrator the commit, tests read it.
- Anything in src/, crates/, tests/, Cargo.* or build.rs keeps its worktree, however small (a half-done edit there
  breaks every build of the main checkout): `cowtree add -b <branch> /Volumes/Worktrees/<branch> origin/main` on the
  Worktrees volume (never touch /Volumes/Worktrees/warp, cowtree's donor, /Volumes/Worktrees/integrator, nor the
  user's own /Volumes/Worktrees/me, which prune-worktrees.sh never removes). A new worktree's files get their last
  commit's time (~/.git-hooks/post-checkout runs ~/dev/bin/restore-mtimes), not the checkout time.
- On the Grok box (Debian, /workspace) worktrees come from `boxtree <branch> [base]` (default origin/main): a
  copy-on-write copy of /workspace/cow/warp, target/ build cache included, at /workspace/cow/trees/<branch> on a new
  branch <branch>. Remove your own with `boxtree rm <branch>` after landing. Never work in /workspace/cow/warp itself.
  Run compilers with `WARP_LIB=<tree>/lib`. The shared box binary is /workspace/cow/warp-rust-latest (built from main;
  refresh with `/workspace/box-slot.sh /workspace/build-fresh-warp.sh`, which moves the link only after a passing
  smoke test); a branch with Rust changes uses its own build. Branch self-check: `/workspace/box-slot.sh
  /workspace/selfcheck-branch.sh <branch>`.
- Only the pure pipeline counts (user, 2026-10-23): coverage is what passes purely; the Rust compiler's results don't
  matter. Don't cargo-build `warp` per branch: run `/workspace/box-slot.sh /workspace/cow/warp-rust-latest rebuild pure`
  with `WARP_LIB=<tree>/lib`. Rebuild Rust only on branches that change Rust code (src/ is frozen except urgent fixes);
  crates/warp-runtime changes need only a warp-host build. The merger refreshes the shared binary after Rust lands.
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
- PAUSED : no cloud runs
  (scripts/cloud-test.sh refuses while ~/.config/warp/cloud-paused exists), GitHub workflows disabled, the context-swap
  hook swaps nobody (~/.claude/swap/swap_tokens = 1e9), crons urgent-card-watch, codex_worker.sh and nightly.sh
  commented out (#PAUSED-by-user). Until the user lifts it, only warp-only branches land (self-check), Rust branches
  wait.
- Rust tests run in the cloud :
  commit, push, `scripts/cloud-test.sh` (below); no local tests/queue.sh runs (a hook blocks them), except tests
  needing the Mac (sound, GPU, windows, Lean, SDL: scripts/cloud-test-environmental.txt), written `MAC_ONLY_TESTS=1
  tests/queue.sh -- <filter>`. The Integrator's full suite runs in the cloud too. Warp tests (`warp test`,
  scripts/is_tests.py) run locally with the shared binary.
- Stop isolation : an agent stops only its own helper jobs, never with a stop that
  reaches other agents.
- Communicate less : message only when it really matters (Ready, blocked, a decision needed); batch
  the rest and send FYIs without waking the agent. Git is the status: branch name, commit messages and the board
  card. Ready means the card moved and the branch pushed, no chat needed.
- Low effort for routine runs : routine builds and tests (rebuilds, targeted test runs, self-checks,
  gate runs) go to a helper at low effort with a step cap; give it the exact commands and stop when they finish.
  Higher effort only for real debugging or design work.
- Tiered gate, pure pipeline first : `scripts/gate-probes.sh
  [--tiers 0-2] [--base <ref>] [<tree>] [<binary>]` stops at the first red tier. Tier 0: rebuild pure (warp.wasm) and
  the hellos (minutes; self only when data/ has a promoted warp-self.wasm). Tier 1: every probes/gate/<name>.warp in
  those lanes against <name>.expected
  (seconds; the set grows by adding the two files, the expected output is the right value, never today's error).
  Tier 2: `warp test` (pure) of only the test files changed since --base (`--folders`: their whole folders). Tier 3,
  Rust tests: only when the change touches src/, crates/, *.rs, Cargo.*, build.rs; the script names it, the cargo
  gate or cloud lane runs it. On the box: through /workspace/box-slot.sh, shared binary /workspace/cow/warp-rust-latest (the old sol-warp-* names stay as links for old scripts).
- warp-self.wasm (`warp self`'s compiler) is no gate build: `scripts/promote-self.sh [--background] [<tree>]` copies
  warp.wasm to it only when warp.wasm compiles compiler.warp to its own bytes (the fixpoint), every once in a while,
  in parallel; it never fails its caller. Install a compiler pair as warp.wasm + warp-self.wasm (the old names
  compiler.wasm / compiler-self.wasm are still read when the new ones are missing).
- Known-good tags and lane bisects: after a green batch gate on main, `scripts/gate-probes.sh --tag-good` tags it
  `good/<YYYY-MM-DD-HHMM>` (Berlin time) and pushes the tag. A lane regression: `scripts/bisect-lane.sh [--lane
  pure|self|both] <probe.warp> <expected> [good] [bad]` (default the newest good/* tag .. origin/main, first-parent)
  bisects in a temporary worktree it removes: per step rebuild pure (and the self promotion) with the shared binary, bad only on
  wrong output, skipped (125) on a failed rebuild or a timeout; it prints the first bad commit.
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
  One exception : warp-speed may build its own branches locally (quick
  release, CARGO_BUILD_JOBS=2, one build at a time) to time Rust speed changes before they land; timing runs only,
  tests stay in the cloud.
- Long commands in the background : anything expected to take more than a few seconds (cloud
  lane, builds, surveys, stage compiles, sweeps) runs with the Bash tool's `run_in_background`, so messages reach you
  while it runs. Never poll its output file or sleep-loop on it: the exit notice wakes you; meanwhile do other work
  or end the turn. Note the job's PID in your handover; a job you abandon gets killed by PID.
- Close what you start : after every test, probe or build, kill your own leftover processes by PID
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
- Tests are result-based (user, 2026-10-23): assert what a program computes, e.g. `is! '6*7' 42`, or its printed
  output. Don't pin incidental details such as exit codes of successful runs, internal forms, pass names, error
  wording or timing. Exit codes are tested only when the feature itself is about them (e.g. `exit(5)`).
- Native-only test code: anything used only by a `#[cfg(feature = "native")]` test gets the same cfg; a whole test
  file running the CLI or std::os::unix gets it on its `mod` line, and a new `mod x;` never goes between a cfg line
  and the module it guards. Register a new test file in its folder's mod.rs before `cargo fmt` (fmt skips it).
- Cloud test lane: `scripts/cloud-test.sh --card <card> <branch> <tip from git rev-parse HEAD> <filters>` in the
  background (~6 min on a cloud VM of its own, ~$0.30 of the Cloud session credit, exit 0 = green;
  notes/cloud_development.md). A green cloud run counts as the targeted run. A Rust change to the parser, lowering,
  analyzer, emitter, compiler runner, or pipeline also runs `scripts/cloud-test.sh --pure-hello <branch> <tip>`: the
  preset gives the ignored end-to-end `warp pure hello` test its required test opt-level 2 and exact runner flags.
  The `warp self` tests (warp_self_*) run with `--optimized`, the gate's opt-level 2: in a debug build they time out.
  Warp-only branches (user 2026-10-21, decision warp-only-acceptance: nothing in src/, crates/, tests/**/*.rs,
  Cargo.*, build.rs) need no cloud run at all: the branch is accepted when the Rust-compiled compiler.warp compiles
  itself and that self build compiles and runs hello world: `scripts/self-check.sh <worktree>` (main's binary; it
  queues behind a running check, one at a time on the Mac: never run `warp rebuild self` in parallel). Mark it
  `todo ready <card> <branch> --express`; the Integrator re-runs the check on the merged tip and pushes. While
  main's own self build is red (its owner fixes it), a branch passes when its pure lane is green and its self check
  fails no worse than main's (the same error): a red main never blocks the queue (supervisor 2026-10-22). Warp
  branches have priority over Rust-change branches : they land before any Rust batch, and work in
  .warp is preferred whenever a task can be done there.
  A change to lib/compiler/ (types or annotations included) also checks the self build before Ready:
  `scripts/self-check.sh <worktree>` (it rebuilds both compilers in a queue slot; a bare `warp rebuild self` skips the
  queue: three at once pushed the load to 288, supervisor 2026-10-22). Never rebuild in the main checkout: that
  replaces main's delivered pair under every running check; only the Integrator's push installs it; a form
  emitter.warp refuses there is fixed or carded, never left for main to find.
  Fleet-wide at most 2 heavy warp runs at once : every self build, `warp rebuild`, `warp
  pure` of a compiler file and every test sweep (is_tests, is_pairs, probe series) runs inside `scripts/slot.sh <cmd>`
  (slots on /Volumes/Build/self-check; self-check.sh takes one itself; the Integrator's gate included). A sweep runs
  its programs serially inside one slot, never N parallel warps. The Integrator's gate runs with WARP_SLOT_GATE=1 in
  a reserved slot of its own, so it never waits behind workers. The Grok box (Debian, 8 cores, 15 GB) has 3 heavy
  slots of its own: every heavy box run goes through `/workspace/box-slot.sh <cmd>` (slots 2-3 open only with >= 4 GB
  free); Mac sessions hand box runs to the Supervisor (grok-msg). The tree it checks
  is a cowtree worktree under /Volumes/Worktrees or a throwaway lib/ copy under /Volumes/Junk, never inside the main
  checkout (scratch/ copies there load fseventsd; the script refuses them). Heavy self-build series (bisecting,
  cold module caches) go to Grok Warp Code 3's Linux box.
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
- `warp pure` / `warp self` run compiler.warp's build files warp.wasm / warp-self.wasm (decision
  compiler-build): next to the binary for the main checkout's lib/, in <worktree>/data/ for a worktree's
  (`WARP_LIB=<worktree>/lib`). `warp pure` / `warp self` never build one : a missing one is an error
  naming `warp rebuild`; a worktree's stale one in data/ gets a one-line notice, the pair beside the binary none. Nothing rebuilds a compiler because lib/ changed (user 2026-10-22: "we don't want automatic
  rebuilds just by changing of lib"): no cron, no git hook, no watcher. The pair beside the binary is main's
  delivered compiler: the Integrator's push builds both from the pushed tip's committed lib/ and installs them
  together (notes/agents/integrator.md). A worktree's
  pair is yours: after editing lib/, run `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp rebuild pure` before
  judging `warp pure`.
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
- Backlog cards : Soon and Later mix the user's own ideas and bugs with auto-generated and parked
  cards. Before taking a card from Soon or Later, check it's still relevant: is it already fixed on main, a duplicate,
  or obsolete (e.g. Rust-only work)? If it's clearly dead, archive it with a one-line note. If unsure, ask the user,
  interviewing them briefly (what's the card for, is it still wanted) before starting work.
- Picking cards : never read whole columns or the board to choose work. Run `todo pick` (or the top
  of the column in board order): the top few cards of the highest non-empty column, in board order. Take the first
  one that's still relevant; the board order is the priority.
- The board's mechanics (topics, sub-cards, keys, renames on take, `--keep-name`, `--title`) are in `todo --help`.
  Every card has a meaningful key: rename a poor one at once (`todo key <card> <short-name>`) and use the key in
  branch names and messages. Priority lives at the topic level (a topic card's column).
- Every branch belongs to a card : when you start a branch, comment the branch name on its card
  (create the card first if there is none); when you park a branch, write a handover file and link it from the card.
  The comment names the branch, its tip, ahead/behind main, whether it merges cleanly, its state in one sentence and
  the next step; an issue card gets an issue comment, a draft card the same lines appended to its body. Branch
  handovers live in the repo, one file per branch: notes/handover/<branch>.md (a `/` in the name becomes `-`), with
  what's done, what's missing, how to test it and its known failures; whoever picks the branch up reads it first and
  updates it. Session handovers stay in notes/box/handover/<session>.md.
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
     Keep merges real: one merge commit per branch, and never let a plain `git pull --rebase` flatten them (it replays
     a merged branch's commits onto main and conflicts): `git pull --rebase=merges` or `git merge origin/main`.
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

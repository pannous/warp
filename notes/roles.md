# Agent roles in warp

Startup prompts and the spawn script for every role: notes/agents/ (README.md says how to start the team).

Decided by the user 2026-10-03, after several sessions ran whole test binaries at once (load 70–130 on 10 cores) and
the supervisor did all merging and testing itself: "spawn a test agent that does everything related to the test",
"let every agent know that they should just delegate their checks to the tester", "an integrator or merger … you should
be free of these tasks and just do high-level supervision".

Tester and integrator are ONE role (the Integrator): the full suite only means something on the merged tree, so a
separate merger would hand every merge to the tester and wait; one session doing merge → test → push has no handoff.
The user asked "I don't see the tester" and agreed ("Makes sense"). Split a separate Tester off only if the Integrator
becomes a bottleneck: it would take targeted-check requests and flaky-test hunts, the Integrator would keep merging.

Current sessions (2026-10-03): Supervisor = BOSS-cheeky-shannon (was warp-43), Integrator = warp-6c (spawned as
warp-integrator). Session names can change; ListAgents shows the current ones.

## Supervisor
High-level only: talks to the user, turns decisions into tasks (notes/open_decisions.md, the board
via ~/dev/bin/todo), spawns and assigns workers, watches load. No merging, no test runs, no decision questions
to the user (those go to the Interviewer).

## Interviewer
User 2026-10-03: "User decisions must never be blocking. Create a new role. Interviewer. That has the only right to
ask me decision questions."
- The only session that asks the user decision questions (multiple choice with a recommended option, AskUserQuestion).
  Every other session sends its question to the Interviewer instead and never waits for the answer.
- Keeps the queue: "## Pending questions" at the top of notes/open_decisions.md (question, options with the
  recommendation first, the assumption already taken, who asked, which branch/test it touches).
- Asks in batches of up to 4, never waiting to be prompted. User 2026-10-03: "since you're only task is to interview me
  you don't need to wait just start the interview whenever you have a batch of questions". Records each answer
  verbatim in the Decided section with the date, then tells the asking session (and the Supervisor) the answer, so the
  assumption is kept or reverted.
- Merges duplicates, drops questions that the code or an earlier decision already answers, and orders by impact.
- Ready to try (user 2026-10-10: "a general mechanism when some feature is done and ready to be tested by me"; the
  question popups are the notifications that reach the user): after each batch of questions, and whenever idle,
  `todo try --new` lists finished features not announced yet. Try lines spell out the full path
  /Users/me/dev/bin/warp: main built with the optimized quick-release profile (Cargo.toml), rebuilt in the background
  by refresh_main_binary.sh a few minutes after every pushed batch (`--version`: "…-main"; log:
  warp.worktrees.noindex/main_binary.log); main's debug build is /Users/me/dev/bin/warp-debug. Plain `warp` and a sample's `#!/usr/bin/env warp` run brew's copy (/opt/homebrew/bin
  comes first on PATH), so a sample is tried as `/Users/me/dev/bin/warp samples/x.warp`. Homebrew releases are occasional (user,
  2026-10-10), so nothing waits for brew; a feature is announced once its batch is pushed (workers run `todo done`
  only after their branch is on main). Never the shared-target debug/warp: it holds whichever worktree built last.
  Format (user 2026-10-10): the Interviewer's FINAL reply of a turn (the user's app summarizes earlier text and
  shows only the final message in full) lists each feature as a ```sh code block the user runs with its ▶ button,
  the expected result after it; no popup follows. Shape (user): `warp 'a = 1; b = 2; a + b'`, the program right
  after `warp`, statements joined by semicolons, no echo/printf pipes or newlines. The user's interactive shell finds
  /Users/me/dev/bin/warp first, so plain `warp` means main's build; a branch-only feature names its worktree binary
  in the same shape. The user answers in their next message: works → `todo tried
  <card>`; a failure → `todo add "…" Now` for the worker who built it; untouched ones stay in `todo try`. Fallback:
  the command in a popup question's text or preview. The Interviewer runs every snippet itself first and announces
  only the ones that pass. Speed (user: "if it runs the test just for this one feature, we could
  already present it"): a worker may hand over a try line as soon as its targeted test passes on its branch, with
  its own binary (scripts/own-warp.sh → <worktree>/scratch/warp), not waiting for the Integrator's batch.
  A popup blocks only the Interviewer.

## Never blocked by a decision
A session that needs a decision: take the recommended option (the one most in line with notes/welcoming.md and the
decided rules), mark it in code/notes as an assumption, keep working, and SendMessage the Interviewer the question.
If the answer differs, the change is undone or redone as a normal small task. Edits to existing tests count as such a
decision too: make the edit on the branch in its own commit (named in the message), so reverting it is one revert;
the Integrator merges it and the Interviewer gets the question.
User, 2026-10-05: "it's allowed to un ignore test that are suddenly passing": dropping `#[ignore]` from a test that
passes unedited needs no question; editing its assertions still needs a decision.

## Integrator (also the Tester)
The only session that runs the full suite and the only one that pushes code to main.
- Keeps one integration worktree (/Volumes/Worktrees/integrate, detached, follows origin/main;
  created with `cowtree add --detach <path> origin/main`).
- Its queue is the board's Ready column (user, 2026-10-16): workers push and run `todo ready <card>`, which records
  branch and tip and refuses a branch that does not merge cleanly into origin/main; `todo list Ready` shows them. Only
  Ready cards are merged; `todo ready --stray` reports unmerged branches without a card (never merged). It merges
  the tips (resolving only trivial conflicts: formatting, mod/use lists, adjacent list additions; logic touched on
  both sides bounces to the owner), runs `./test.sh` with the integration build tweak, and pushes to main only at 0
  failed and no drop in the test count, then `todo done <card> <commit>` (Ready → Done). Otherwise the card bounces:
  `todo move <card> Now` + `todo note` with the failure (and the supervisor hears if it's a decision).
- Fix forward, don't hold the batch (user, 2026-10-16): when up to 10 tests fail after a merge, the Integrator marks
  each one `#[ignore = "fixnow: <card>: <first panic line>"]` in its own commit and pushes the batch; for each test
  it adds a `fixnow-<test>` card in Now for the owner of the branch that broke it (or the area worker). The owner
  fixes forward and drops the ignore; `grep -rn 'fixnow' tests/` lists what's open. When the ignored test pins an
  old edge case the new behaviour arguably improves on, the owner doesn't force the old one back: the question
  goes to warp-interviewer (keep the old case, or change the test to the new one), the ignore stays until answered. Only more than 10 failures,
  or a failure nobody can fix forward (wrong output across a whole feature, a crash at start), leads to real
  backtracking: the branch leaves the batch and bounces. The test-count check counts fixnow ignores as present.
- Batches of at most ~8 Ready cards per run; wide refactors and carrier branches land alone and first. Runs a
  worker's targeted check on request.
- Express (user, 2026-10-10: "faster results for an agent"): a card readied with `--express` (the user asked for that
  agent's result) is not batched: it goes next, alone on main, its full suite ahead of the batch run (a running suite
  finishes first). It skips the waiting, never the suite: one red branch on main stops every worker. To try a result
  even sooner, the worker's early-try binary (notes/agents/common.md) needs no merge at all.
- Crash fixes first (user, 2026-10-12: "Crash fixes must be merged with highest priority"): a crash card or a branch
  fixing a panic or trap is always express, alone and next, ahead of any queued batch (a running suite finishes first).
- User changes are signals (user, 2026-10-12): whenever a batch or a push meets changes by the user (commits on main,
  uncommitted edits carried along), look for the wish behind them and a general pattern: a commented-out line or
  TODO is a missing feature, a changed constant a complaint (speed, size), a new sample a use case. Each becomes a
  card (`todo add`), or a question for the Interviewer when the intent is open.
- Every landing gets a try line (user, 2026-10-12; notes/agents/common.md): the merge report names each user-visible
  branch to its owner, who sends warp-interviewer the try line; an owner that has retired gets none, so the
  Integrator sends that branch's try line itself.
- Before every merge: Cargo.toml/Cargo.lock restored (a branch that changes Cargo.toml is otherwise refused, and a
  grep for "conflict" does not show that). Before reporting a branch merged: `git merge-base --is-ancestor
  origin/<branch> HEAD`.
- Triggers GitHub CI only when asked or for a release (`gh workflow run "Rust CI" -R pannous/warp`).
- Keeps the baseline (test_results.txt) and the "Merged" lines on the board; tells the supervisor after each merge.

## Reviewer
User 2026-10-14 invented it: "makes sure that everything is sound and beautiful according to standards", so the
Integrator has less to do. It reads each Ready card's diff in parallel with the gate: two signals for the
worker (user 2026-10-15), the gate's "it works" and the Reviewer's "here it can generalize or read better", which the
worker makes its next commit. The Reviewer approves nothing and sends the Integrator nothing but a rare veto
(unsound: the card goes back to Now with a note, the Integrator leaves it out of the push).
Structural findings become cards for the Refactorer. Prompt: notes/agents/reviewer.md; a default service.

## Refactorer
User 2026-10-14: "a new role refactorer that will regularly reorganize the codebase by keeping it structural and
generalized and beautiful". A long-running session, not a default service: the Supervisor starts it with
`notes/agents/start.sh refactorer` and wakes it after a batch of feature merges or once a day. It moves functions to
their concept's file, merges duplicate and similarly named functions, and folds special cases into the general
machinery. Every chunk goes on its own branch to the Integrator, stating its net src/ delta. Feature cuts go to the
Interviewer. Prompt: notes/agents/refactorer.md.

## Workers
- One task, one branch, one worktree: `cowtree add -b <name> /Volumes/Worktrees/<name> origin/main`, on the Worktrees
  volume (user 2026-10-16, notes/churn_volume.md: Spotlight off, no fseventsd log, cowtree clones the donor
  /Volumes/Worktrees/warp, which nobody removes). Outside the repo, so grep/IDE/cargo of the main checkout never see it.
  Older worktrees in /Users/me/dev/angles/warp.worktrees.noindex stay there until they retire.
  Uncommitted build tweak in it: `version = "0.1.1-<name>"` (notes/build_speed.md).
- Small changes on main (user, 2026-10-12, "new very important rule": "Agents can work on the main branch without
  work tree if their changes are small or not conflicting with other work"; narrowed the same day after "maybe my new
  rule is stupid if their changes block all the tests"): only changes that cannot break the build or a test, i.e.
  nothing compiled: notes, docs, wiki, probes, scripts, samples, playground text, go straight onto main in the main
  checkout /Users/me/dev/angles/warp, no worktree or branch. Anything in src/, crates/, tests/, Cargo.* or build.rs
  keeps its worktree, however small: a half-done edit there breaks every build of the main checkout (the user's and
  every session's working on main). There: commit your own paths in their own commit (`git commit -- <paths>`), and
  the user's uncommitted edits you meet are integrated too, not left lying (user, 2026-10-12: "integrate and commit
  my changes instead of leaving them there"): read each diff, fix what it breaks (a deleted sample's references, a
  test it changes the meaning of goes to the Interviewer), commit it as the user's change in a commit of its own; run
  the targeted tests through the queue, `git pull --rebase --autostash`, push, and tell the Integrator the commit and
  filters (its next full run is the check). Bigger or overlapping work keeps its worktree and branch.
  Auto-commit, right away and in the main checkout itself (user, 2026-10-13: "always auto-commit my changes and
  integrate them in a meaningful way"): staged and untracked files count too; the user's `git status` must end clean.
  Never park their edits only in an Integrator batch: an unpushed batch commit leaves their checkout showing them
  uncommitted. Meaningful: complete them to the conventions (a new sample gets its shebang and chmod +x), run them; if
  the intent is unclear, ask warp-interviewer.
  Moving on demand: a change that grows (more than a few files, many build/test rounds, or another session's files)
  moves into a worktree while still uncommitted: `cowtree add -b <task> <worktree> HEAD`, `git diff -- <own paths> |
  git -C <worktree> apply`, then undo only your own paths in the main checkout. Never leave unpushed commits on main.
  The way back: a branch that turns out small goes straight to main itself (`git pull --rebase` in the worktree,
  push `HEAD:main`), no Integrator batch. Why not everything on main: sessions there build and test each other's
  half-done edits.
- Test first, then implement. While developing, only targeted tests and only through the queue:
  `tests/queue.sh -- <filter>`. Never the whole tests binary, never an extra export copy for verification.
- Cloud test lane (card cloud-test-lane, 2026-10-16; notes/cloud_development.md section 5): a pushed tip's targeted
  tests can run on a cloud VM of their own instead of the Mac's queue:
  `scripts/cloud-test.sh --card <card> <branch> <tip from git rev-parse HEAD> <filters>`, in the background (~6 min,
  ~$0.30 of the Cloud session credit; exit 0 = green, the result goes on the card). Use it when more than a few runs
  wait in the queue (`ls ~/.cargo/warp-tests.lock.queue | wc -l`) and the change is committed and pushed. Keep
  tests/queue.sh for uncommitted edits, quick re-runs of one test, and tests that need the Mac (sound devices, GPU,
  windows, Lean, SDL: scripts/cloud-test-environmental.txt). A green cloud run counts as the targeted run before
  `todo ready`; the Integrator's gate stays the check. At most 3 cloud runs at once (the script waits for a free one).
- Done = commit on the branch, targeted tests, merge origin/main (conflicts are the worker's to resolve), push the
  branch, `todo ready <card>` (notes/agents/common.md "Done"). The Integrator's full-suite result is the check; a
  bounced card comes back to Now with a note, the worker fixes it and readies it again.
- Closing a card whose result the user can see or use (a feature, a visible fix, a sample, a playground change):
  `todo done <card> <commit> --try "how to try it"`: one concrete line (a command, a playground snippet, a sample
  file), only once it is on main and, for web changes, deployed. Internal work (cleanup, tests, notes) has no --try.
- After the merge, remove your worktree and branch: `git -C /Users/me/dev/angles/warp worktree remove --force <worktree> && git -C /Users/me/dev/angles/warp branch -D <branch>` (the hook allows both when only the build tweak is uncommitted and main or a remote holds the commits; notes/agents/common.md). Write the paths literally: the hook cannot check a shell variable.

## Branches, copies and worktrees
A worktree is a second checkout of the same repository: same .git object store, its own branch, `git status` and
commits work as usual, no clone or archive needed. The earlier "copies" (git archive exports, probes/*_work) were
plain directories without git: committing from them needed private index tricks and they piled up inside the repo.
So branches are official, and every branch lives in a worktree under /Volumes/Worktrees/. The main checkout
/Users/me/dev/angles/warp stays the user's own and agents do not edit it.

## Enforcement
- `tests/queue.sh`: every test run waits for one of 3 machine-wide slot locks (`~/.cargo/warp-tests.lock`, `.lock.2`,
  `.lock.3`, lockf), so at most 3 targeted runs go at once (4 test threads each, nice 10); a full `./test.sh` holds all
  three and runs alone, and re-enters through it. Each slot's holder is in its `.owner` file (`warp-tests.lock.owner`
  for slot 1). Waiters are served first come, first served by ticket files in `~/.cargo/warp-tests.lock.queue/` (named
  `<priority>-<time>-<pid>`, tickets of dead PIDs are dropped, a run's ticket goes once it holds its slots): only the
  first in line takes a slot, so a waiting full run (priority 0, like the Integrator's integrate* targeted runs) keeps
  any later run from starting until the slots drain. Plain lockf alone was not FIFO: a full run waited 15 min behind
  workers; one slot alone left the Mac at load 7 of 10 with 37 runs queued (card test-queue-slots).
- `.claude/hooks/test-gate.py` (PreToolUse, Bash): blocks `cargo test`, `cargo browser-test` and direct `deps/tests-*`
  binaries that bypass the queue; `cargo test --no-run` (build only) passes. It applies to every session whose project
  directory is this checkout, which includes all spawned workers.
- Both are Mac-local: cloud sessions (`CLAUDE_CODE_REMOTE=true`, a VM of their own) are exempt from the gate, and
  queue.sh falls back to flock on Linux, or runs unlocked with a loud note when neither lockf nor flock exists.
- Not covered: a script a worker writes itself that calls cargo test, and sessions in other checkouts; the role rule
  covers those.

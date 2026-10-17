# Agent roles in warp

Startup prompts and the spawn script for every role: notes/agents/ (README.md says how to start the team).
ListAgents shows the current sessions; each is named `warp-<role>`.

Tester and integrator are ONE role (the Integrator): the full suite only means something on the merged tree, so one
session doing merge → test → push has no handoff. Split a separate Tester off only if the Integrator becomes a
bottleneck: it would take targeted-check requests and flaky-test hunts, the Integrator would keep merging.

## Supervisor
High-level only: talks to the user, turns decisions into tasks (notes/open_decisions.md, the board via
~/dev/bin/todo), spawns and assigns workers, watches load. No merging, no test runs, no decision questions to the user
(those go to the Interviewer).

## Interviewer
User decisions never block. The Interviewer is the only session that asks the user decision questions (multiple
choice with a recommended option, AskUserQuestion). Every other session sends its question to the Interviewer and
never waits for the answer.
- Keeps the queue: "## Pending questions" at the top of notes/open_decisions.md (question, options with the
  recommendation first, the assumption already taken, who asked, which branch/test it touches).
- Asks in batches of up to 4 as soon as it has a batch, never waiting to be prompted. Records each answer in
  notes/decisions.md, then tells the asking session (and the Supervisor), so the assumption is kept or reverted.
- Merges duplicates, drops questions the code or an earlier decision already answers, orders by impact.

### Ready to try
After each batch of questions, and whenever idle, `todo try --new` lists finished features not announced yet.
- Binaries: /Users/me/dev/bin/warp is main built with the optimized quick-release profile, rebuilt in the background
  by refresh_main_binary.sh a few minutes after every pushed batch (`--version`: "…-main"; log:
  warp.worktrees.noindex/main_binary.log); main's debug build is /Users/me/dev/bin/warp-debug. Plain `warp` and a
  sample's `#!/usr/bin/env warp` run brew's copy for agents (/opt/homebrew/bin comes first on their PATH), so a sample
  is tried as `/Users/me/dev/bin/warp samples/x.warp`; the user's interactive shell finds /Users/me/dev/bin/warp first.
  Never the shared-target debug/warp: it holds whichever worktree built last. Homebrew releases are occasional, so
  nothing waits for brew.
- A feature is announced once its batch is pushed (workers run `todo done` only after their branch is on main). A
  worker may hand over a try line as soon as its targeted test passes on its branch, with its own binary
  (scripts/own-warp.sh → <worktree>/scratch/warp).
- Format: the Interviewer's FINAL reply of a turn (the user's app shows only the final message in full) lists each
  feature as a ```sh code block with the expected result after it; no popup follows. Shape: `warp 'a = 1; b = 2; a + b'`,
  the program right after `warp`, statements joined by semicolons, no echo/printf pipes or newlines; a branch-only
  feature names its worktree binary in the same shape. The Interviewer runs every snippet itself first and announces
  only the ones that pass.
- The user answers in their next message: works → `todo tried <card>`; a failure → `todo add "…" Now` for the worker
  who built it; untouched ones stay in `todo try`.

## Never blocked by a decision
A session that needs a decision takes the recommended option (the one most in line with notes/welcoming.md and the
decided rules), marks it in code/notes as an assumption, keeps working, and SendMessages the Interviewer the
question. If the answer differs, the change is undone or redone as a normal small task. Edits to existing tests count
as such a decision: the edit goes on the branch in its own commit (named in the message), so reverting it is one
revert. Dropping `#[ignore]` from a test that passes unedited needs no question; editing its assertions still does
(upgrades from "not yet" to the working value are allowed: AGENTS.md "Standing permission").

## Integrator (also the Tester)
The only session that runs the full suite and the only one that pushes code to main.
- Keeps one integration worktree (/Volumes/Worktrees/integrate, detached, follows origin/main;
  created with `cowtree add --detach <path> origin/main`).
- Its queue is the board's Ready column: workers push and run `todo ready <card>`, which records branch and tip and
  refuses a branch that does not merge cleanly into origin/main; `todo list Ready` shows them. Only Ready cards are
  merged; `todo ready --stray` reports unmerged branches without a card (never merged). It merges the tips (resolving
  only trivial conflicts: formatting, mod/use lists, adjacent list additions; logic touched on both sides bounces to
  the owner), runs `./test.sh` with the integration build tweak, and pushes to main only at 0 failed and no drop in
  the test count, then `todo done <card> <commit>` (Ready → Done). Otherwise the card bounces: `todo move <card> Now`
  + `todo note` with the failure (and the Supervisor hears if it's a decision).
- Fix forward, don't hold the batch: when up to 10 tests fail after a merge, the Integrator marks each one
  `#[ignore = "fixnow: <card>: <first panic line>"]` in its own commit and pushes the batch; for each test it adds a
  `fixnow-<test>` card in Now for the owner of the branch that broke it (or the area worker). The owner fixes forward
  and drops the ignore; `grep -rn 'fixnow' tests/` lists what's open. When the ignored test pins an old edge case the
  new behaviour arguably improves on, the question goes to warp-interviewer (keep the old case, or change the test),
  the ignore stays until answered. Only more than 10 failures, or a failure nobody can fix forward (wrong output
  across a whole feature, a crash at start), leads to backtracking: the branch leaves the batch and bounces. The
  test-count check counts fixnow ignores as present.
- Batches of at most ~8 Ready cards per run; wide refactors and carrier branches land alone and first. Runs a
  worker's targeted check on request.
- Express: a card readied with `--express` (the user asked for that agent's result) is not batched: it goes next,
  alone on main, its full suite ahead of the batch run (a running suite finishes first). It skips the waiting, never
  the suite: one red branch on main stops every worker.
- Crash fixes first: a crash card or a branch fixing a panic or trap is always express.
- User changes are signals: whenever a batch or a push meets changes by the user (commits on main, uncommitted edits
  carried along), look for the wish behind them and a general pattern: a commented-out line or TODO is a missing
  feature, a changed constant a complaint (speed, size), a new sample a use case. Each becomes a card (`todo add`),
  or a question for the Interviewer when the intent is open.
- Every landing gets a try line: the merge report names each user-visible branch to its owner, who sends
  warp-interviewer the try line; for an owner that has retired the Integrator sends it itself.
- Before every merge: Cargo.toml/Cargo.lock restored (a branch that changes Cargo.toml is otherwise refused, and a
  grep for "conflict" does not show that). Before reporting a branch merged: `git merge-base --is-ancestor
  origin/<branch> HEAD`.
- Triggers GitHub CI only when asked or for a release (`gh workflow run "Rust CI" -R pannous/warp`).
- Keeps the baseline (test_results.txt) and the "Merged" lines on the board; tells the Supervisor after each merge.

## Reviewer
Makes sure everything is sound and beautiful according to standards. It reads each Ready card's diff in parallel
with the gate: two signals for the worker, the gate's "it works" and the Reviewer's "here it can generalize or read
better", which the worker makes its next commit. The Reviewer approves nothing and sends the Integrator nothing but a
rare veto (unsound: the card goes back to Now with a note, the Integrator leaves it out of the push). Structural
findings become cards for the Refactorer. Prompt: notes/agents/reviewer.md; a default service.

## Refactorer
Regularly reorganizes the code base, keeping it structural, general and beautiful. A long-running session, not a
default service: the Supervisor starts it with `notes/agents/start.sh refactorer` and wakes it after a batch of
feature merges or once a day. It moves functions to their concept's file, merges duplicate and similarly named
functions, and folds special cases into the general machinery. Every chunk goes on its own branch to the Integrator,
stating its net src/ delta. Feature cuts go to the Interviewer. Prompt: notes/agents/refactorer.md.

## Workers
- One task, one branch, one worktree: `cowtree add -b <name> /Volumes/Worktrees/<name> origin/main`, on the Worktrees
  volume (notes/churn_volume.md: Spotlight off, no fseventsd log, cowtree clones the donor /Volumes/Worktrees/warp,
  which nobody removes). Outside the repo, so grep/IDE/cargo of the main checkout never see it. Older worktrees in
  /Users/me/dev/angles/warp.worktrees.noindex stay there until they retire. Uncommitted build tweak in it:
  `version = "0.1.1-<name>"` (notes/build_speed.md).
- Small changes on main: only changes that cannot break the build or a test, i.e. nothing compiled: notes, docs,
  wiki, probes, scripts, samples, playground text, go straight onto main in the main checkout
  /Users/me/dev/angles/warp, no worktree or branch. Anything in src/, crates/, tests/, Cargo.* or build.rs keeps its
  worktree, however small: a half-done edit there breaks every build of the main checkout. There: commit your own
  paths in their own commit (`git commit -- <paths>`), run the targeted tests through the queue,
  `git pull --rebase --autostash`, push, and tell the Integrator the commit and filters (its next full run is the
  check). Never leave unpushed commits on main.
- The user's uncommitted edits met in the main checkout are integrated at once, in the main checkout itself, not
  left lying or parked in an Integrator batch (an unpushed batch commit leaves their checkout showing them
  uncommitted); staged and untracked files count too, the user's `git status` must end clean. Read each diff,
  complete it to the conventions (a new sample gets its shebang and chmod +x), fix what it breaks (a deleted
  sample's references; a test whose meaning it changes goes to the Interviewer), run it, commit it as the user's
  change in a commit of its own. Intent unclear → warp-interviewer.
- Moving on demand: a change that grows (more than a few files, many build/test rounds, or another session's files)
  moves into a worktree while still uncommitted: `cowtree add -b <task> <worktree> HEAD`, `git diff -- <own paths> |
  git -C <worktree> apply`, then undo only your own paths in the main checkout. The way back: a branch that turns out
  small goes straight to main itself (`git pull --rebase` in the worktree, push `HEAD:main`), no Integrator batch.
- Test first, then implement. While developing, only targeted tests and only through the queue:
  `tests/queue.sh -- <filter>`. Never the whole tests binary, never an extra export copy for verification.
- Cloud test lane (notes/cloud_development.md section 5): a pushed tip's targeted tests can run on a cloud VM of
  their own instead of the Mac's queue: `scripts/cloud-test.sh --card <card> <branch> <tip from git rev-parse HEAD>
  <filters>`, in the background (~6 min, ~$0.30 of the Cloud session credit; exit 0 = green, the result goes on the
  card). Use it when more than a few runs wait in the queue (`ls ~/.cargo/warp-tests.lock.queue | wc -l`) and the
  change is committed and pushed. Keep tests/queue.sh for uncommitted edits, quick re-runs of one test, and tests
  that need the Mac (sound devices, GPU, windows, Lean, SDL: scripts/cloud-test-environmental.txt). A green cloud run
  counts as the targeted run before `todo ready`. At most 3 cloud runs at once (the script waits for a free one).
- Done = commit on the branch, targeted tests, merge origin/main (conflicts are the worker's to resolve), push the
  branch, `todo ready <card>` (notes/agents/common.md "Done"). A bounced card comes back to Now with a note, the
  worker fixes it and readies it again.
- Closing a card whose result the user can see or use (a feature, a visible fix, a sample, a playground change):
  `todo done <card> <commit> --try "how to try it"`: one concrete line, only once it is on main and, for web changes,
  deployed. Internal work (cleanup, tests, notes) has no --try.
- After the merge, remove your worktree and branch: `git -C /Users/me/dev/angles/warp worktree remove --force
  <worktree> && git -C /Users/me/dev/angles/warp branch -D <branch>` (the hook allows both when only the build tweak
  is uncommitted and main or a remote holds the commits). Write the paths literally: the hook cannot check a shell
  variable.

## Enforcement
- `tests/queue.sh`: every test run waits for one of 3 machine-wide slot locks (`~/.cargo/warp-tests.lock`, `.lock.2`,
  `.lock.3`, lockf), so at most 3 targeted runs go at once (4 test threads each, nice 10); a full `./test.sh` holds all
  three and runs alone. Each slot's holder is in its `.owner` file. Waiters are served first come, first served by
  ticket files in `~/.cargo/warp-tests.lock.queue/` (`<priority>-<time>-<pid>`, tickets of dead PIDs are dropped):
  only the first in line takes a slot, so a waiting full run (priority 0, like the Integrator's integrate* targeted
  runs) keeps any later run from starting until the slots drain. Never rename a ticket.
- `.claude/hooks/test-gate.py` (PreToolUse, Bash): blocks `cargo test`, `cargo browser-test` and direct `deps/tests-*`
  binaries that bypass the queue; `cargo test --no-run` (build only) passes. It applies to every session whose project
  directory is this checkout.
- Both are Mac-local: cloud sessions (`CLAUDE_CODE_REMOTE=true`) are exempt from the gate, and queue.sh falls back to
  flock on Linux, or runs unlocked with a loud note when neither lockf nor flock exists.
- Not covered: a script a worker writes itself that calls cargo test, and sessions in other checkouts; the role rule
  covers those.

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
notes/impl_2026-10-03.md), spawns and assigns workers, watches load. No merging, no test runs, no decision questions
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

## Never blocked by a decision
A session that needs a decision: take the recommended option (the one most in line with notes/welcoming.md and the
decided rules), mark it in code/notes as an assumption, keep working, and SendMessage the Interviewer the question.
If the answer differs, the change is undone or redone as a normal small task. Edits to existing tests count as such a
decision too: make the edit on the branch in its own commit (named in the message), so reverting it is one revert;
the Integrator merges it and the Interviewer gets the question.

## Integrator (also the Tester)
The only session that runs the full suite and the only one that pushes code to main.
- Keeps one integration worktree (/Users/me/dev/angles/warp.worktrees.noindex/integrate, detached, follows origin/main).
- Workers send it "branch, tip, filters"; it merges the branch (union-resolves todo.md / tests/main.rs, sends real
  conflicts back to the worker), runs `./test.sh` with the integration build tweak, and pushes to main only at 0 failed
  and no drop in the test count. Otherwise it reports the failures to the worker (and the supervisor if it's a decision).
- Batches branches that arrive close together into one run; runs a worker's targeted check on request.
- Before every merge: Cargo.toml/Cargo.lock restored (a branch that changes Cargo.toml is otherwise refused, and a
  grep for "conflict" does not show that). Before reporting a branch merged: `git merge-base --is-ancestor
  origin/<branch> HEAD`.
- Triggers GitHub CI only when asked or for a release (`gh workflow run "Rust CI" -R pannous/warp`).
- Keeps the baseline (test_results.txt) and the "Merged" lines on the board; tells the supervisor after each merge.

## Workers
- One task, one branch, one worktree: `git worktree add -b <name> /Users/me/dev/angles/warp.worktrees.noindex/<name>
  origin/main`. Outside the repo, so grep/IDE/cargo of the main checkout never see it, and `.noindex` keeps Spotlight out.
  Uncommitted build tweak in it: `version = "0.1.1-<name>"` (notes/build_speed.md).
- Test first, then implement. While developing, only targeted tests and only through the queue:
  `tests/queue.sh -- <filter>`. Never the whole tests binary, never an extra export copy for verification.
- Done = commit on the branch, push the branch, SendMessage the Integrator "branch, tip, new tests, filters". The
  Integrator's full-suite result is the check; the worker fixes what it reports.
- After the merge, remove the worktree: `rm -rf <worktree> && git -C /Users/me/dev/angles/warp worktree prune` (git worktree remove refuses the dirty build tweak and the hook blocks --force; the branch commits are safe on main).

## Branches, copies and worktrees
A worktree is a second checkout of the same repository: same .git object store, its own branch, `git status` and
commits work as usual, no clone or archive needed. The earlier "copies" (git archive exports, probes/*_work) were
plain directories without git: committing from them needed private index tricks and they piled up inside the repo.
So branches are official, and every branch lives in a worktree under warp.worktrees.noindex/. The main checkout
/Users/me/dev/angles/warp stays the user's own and agents do not edit it.

## Enforcement
- `tests/queue.sh`: every test run waits for one machine-wide lock (`~/.cargo/warp-tests.lock`, lockf), so at most one
  test binary runs at a time; `./test.sh` re-enters through it. The holder is in `~/.cargo/warp-tests.lock.owner`.
  Waiters are served first come, first served by ticket files in `~/.cargo/warp-tests.lock.queue/` (named
  `<priority>-<time>-<pid>`, tickets of dead PIDs are dropped); a full `./test.sh` (the Integrator) has priority 0 and
  takes the next slot ahead of targeted runs. Plain lockf alone was not FIFO: a full run waited 15 min behind workers.
- `.claude/hooks/test-gate.py` (PreToolUse, Bash): blocks `cargo test`, `cargo browser-test` and direct `deps/tests-*`
  binaries that bypass the queue; `cargo test --no-run` (build only) passes. It applies to every session whose project
  directory is this checkout, which includes all spawned workers.
- Both are Mac-local: cloud sessions (`CLAUDE_CODE_REMOTE=true`, a VM of their own) are exempt from the gate, and
  queue.sh falls back to flock on Linux, or runs unlocked with a loud note when neither lockf nor flock exists.
- Not covered: a script a worker writes itself that calls cargo test, and sessions in other checkouts; the role rule
  covers those.

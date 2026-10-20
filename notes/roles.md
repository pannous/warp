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
  /Volumes/Worktrees/integrator/main_binary.log); main's debug build is /Users/me/dev/bin/warp-debug. Plain `warp` and a
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

## Integrator (also the Tester)
The only session that runs the full suite and the only one that pushes code to main. Its queue is the board's Ready
column (`todo ready <card>`); it merges batches of at most ~8 Ready cards in /Volumes/Worktrees/integrate, runs the
gate, pushes at 0 failed (fix forward: up to 10 failures are ignored as `fixnow` onto the owner's card), closes the
cards and bounces the rest to Now with a note. Express cards and crash fixes go live untested at once. How:
notes/agents/integrator.md.

## Reviewer
Makes sure everything is sound and beautiful according to standards. It reads each Ready card's diff in parallel
with the gate: two signals for the worker, the gate's "it works" and the Reviewer's "here it can generalize or read
better", which the worker makes its next commit. The Reviewer approves nothing and sends the Integrator nothing but a
rare veto (unsound: the card goes back to Now with a note, the Integrator leaves it out of the push). Structural
findings become cards for the Refactorer. Prompt: notes/agents/reviewer.md; a default service.

## Steward (warp-steward, the warp compiler's test owner)
Owns "the warp-written compiler passes its own tests", orthogonal to the Integrator, who keeps merging everything.
After each merge batch that touches lib/compiler or the pipeline (and nightly), it runs the warp-pipeline-suite on the
cloud lane, records the pass count over time (data/self_hosting/suite_counts.txt), groups the failures by cause and
turns the biggest groups into cards for the self-hosting workers, reporting the count and the top groups to the
Supervisor. It merges nothing and writes no compiler code beyond the runner itself. Started once the
warp-pipeline-suite runner is on main.

## Refactorer
Regularly reorganizes the code base, keeping it structural, general and beautiful. A long-running session, not a
default service: the Supervisor starts it with `notes/agents/start.sh refactorer` and wakes it after a batch of
feature merges or once a day. It moves functions to their concept's file, merges duplicate and similarly named
functions, and folds special cases into the general machinery. Every chunk goes on its own branch to the Integrator,
stating its net src/ delta. Feature cuts go to the Interviewer. Prompt: notes/agents/refactorer.md.

## Urgent (warp-urgent, in the cloud)
A standing `claude --cloud` session in fast mode that fixes the user's urgent cards; crash cards
keep their local worker. ~/dev/bin/urgent-card-watch sends it each new urgent card (`claude -p … --cloud <id>`, the id
in ~/.cache/todo-board/urgent-cloud-session; SendMessage is held for approval there) and collects its pushed
`claude/urgent-<key>` branches: a `done` report whose tip is on main closes the card, `needs-mac` goes to the
Supervisor. warp-urgent pushes and merges every card to main itself, in any area, without the merge queue; the Integrator's next full run checks it. Prompt: notes/agents/urgent_cloud.md.
Start (or replace) it: `tmux new-session -d -s cloud-warp-urgent -c /Users/me/dev/angles/warp "claude --model opus
--name warp-urgent --permission-mode bypassPermissions --settings '{\"fastMode\": true}' --cloud 'You are warp-urgent. Read notes/agents/urgent_cloud.md
and AGENTS.md, then do the Priming and wait for urgent cards.'"`, then write its session id into that file and
switch fast mode on: `claude -p "/fast on" --cloud <id> < /dev/null`. `--settings '{"fastMode": true}'` does not
reach a cloud session; check the session's footer once.
It must always run in bypass permissions mode: after every start, check
the footer shows bypass permissions; card sends pass `--permission-mode bypassPermissions` too.

## Workers
One task, one branch, one worktree; targeted tests through the queue; done = `todo ready <card>`. Every rule:
notes/agents/common.md (role prompts: worker.md, fixer.md, implementer.md).

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

# Agent roles in warp

Grok Bot agents work a little differently (box, pre-approved plans, status in git): see notes/agents/grok.md.

Startup prompts and the spawn script for every role: notes/agents/ (README.md says how to start the team).
ListAgents shows the current sessions; each is named `warp-<role>`.

Tester and integrator are ONE role (the Integrator): the full suite only means something on the merged tree, so one
session doing merge → test → push has no handoff. Split a separate Tester off only if the Integrator becomes a
bottleneck: it would take targeted-check requests and flaky-test hunts, the Integrator would keep merging.

## Supervisor
Owns the project's progress : talks to the user, turns decisions into tasks (notes/open_decisions.md,
the board via ~/dev/bin/todo), spawns and assigns workers, watches load and metrics. Makes technical and process
decisions itself and may bend a rule that blocks progress, naming it (notes/agents/supervisor.md); language questions
go to the user via the Interviewer.

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
  worker may hand over a try line as soon as its targeted test passes on its branch, with the shared binary
  (`WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp`).
- Format: the Interviewer's FINAL reply of a turn (the user's app shows only the final message in full) lists each
  feature as a ```sh code block with the expected result after it; no popup follows. Shape: `warp 'a = 1; b = 2; a + b'`,
  the program right after `warp`, statements joined by semicolons, no echo/printf pipes or newlines; a branch-only
  feature names its worktree binary in the same shape. The Interviewer runs every snippet itself first and announces
  only the ones that pass.
- The user answers in their next message: works → `todo tried <card>`; a failure → `todo add "…" Now` for the worker
  who built it; untouched ones stay in `todo try`.

## Integrator (also the Tester)
Workers push their own tested merges to main (common.md "Done"); the Integrator repeats no test. It watches main:
installs the compilers beside ~/dev/bin/warp from main's tip, runs tiers 1-2 and promote-self in the background and
the full suite at rare intervals, and turns what goes red into a card for the pusher (a red smoke test on main:
urgent). It also lands branches whose session retired. How: notes/agents/integrator.md.

## Reviewer
Makes sure everything is sound and beautiful according to standards. It reads each Ready card's diff in parallel
with the gate: two signals for the worker, the gate's "it works" and the Reviewer's "here it can generalize or read
better", which the worker makes its next commit. The Reviewer approves nothing and sends the Integrator nothing but a
rare veto (unsound: the card goes back to Now with a note, the Integrator leaves it out of the push). Structural
findings become cards for the Refactorer. Prompt: notes/agents/reviewer.md; a default service.

## Steward (warp-steward, the warp compiler's test owner)
Owns "the warp-written compiler passes its own tests", orthogonal to the Integrator, who keeps merging everything.
At rare intervals (nightly, never per merge batch: the suite is slow), it runs the warp-pipeline-suite, records the pass count over time (data/self_hosting/suite_counts.txt), groups the failures by cause and
turns the biggest groups into cards for the self-hosting workers, reporting the count and the top groups to the
Supervisor. It merges nothing and writes no compiler code beyond the runner itself. Started once the
warp-pipeline-suite runner is on main.

## Refactorer
Regularly reorganizes the code base, keeping it structural, general and beautiful. A long-running session, not a
default service: the Supervisor starts it with `notes/agents/start.sh refactorer` and wakes it after a batch of
feature merges or once a day. It moves functions to their concept's file, merges duplicate and similarly named
functions, and folds special cases into the general machinery. Every chunk goes on its own branch to the Integrator,
stating its net src/ delta. Feature cuts go to the Interviewer. Prompt: notes/agents/refactorer.md.

## Workers
One task, one branch, one worktree; targeted tests (`warp test`); done = pushed to main and `todo done <card> <commit>`. Every rule:
notes/agents/common.md (role prompts: worker.md, fixer.md, implementer.md).

## Enforcement
- One local build (main's, the Integrator's); the Rust harness changes rarely. Warp tests (`warp test`) run locally with the shared binary, the whole suite only in the Integrator's gate.
  `.claude/hooks/test-gate.py` (PreToolUse, Bash) blocks cargo test runs of this checkout.

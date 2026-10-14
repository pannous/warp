# Starting the warp agent team

The rules live in notes/roles.md; this folder holds the startup prompts, one per role, so a fresh day starts the same
way. Proven on 2026-10-03 (about 30 branches merged, suite 1345 → 1525 passed, 0 failed).

## Start
1. Supervisor: `notes/agents/start.sh supervisor` (tmux warp-supervisor). It must run in that tmux session:
   ~/dev/bin/urgent-card-watch delivers new urgent cards only there. After a crash, resume the old one into the same
   tmux name: `FORCE_SPAWN=1 CLAUDE_RESUME=<session id> ~/dev/bin/claude-remote.sh supervisor <repo> "<RESUME prompt>"`.
   A supervisor opened by hand in a terminal misses the urgent cards.
2. The supervisor runs `notes/agents/start.sh`. It spawns the long-running services as Remote Control sessions in
   detached tmux and prints each claude.ai/code URL:
   - warp-integrator (merge, full suite, push to main)
   - warp-interviewer (the only one who asks you decision questions; talk to it when you have a minute)
   - warp-fixer (small to-dos from the board `todo list`, ignored "next"/"soon" tests)
3. At most TWO feature workers at a time (user, 2026-10-06; the 2026-10-06 fleet of eight stalled on the usage limit). Workers are spawned per task by the supervisor with `notes/agents/start.sh worker <name> "<task>"`.

Spawned sessions get generated names (warp-6c …). Find them with ListAgents: the tmux column shows the role
(tmux `warp-<role>`). Two sessions can share a generated name; then address one by its `[ref]`.

## Files
- common.md: rules every spawned session follows (standing instruction, worktrees, test queue, never blocked).
- supervisor.md, integrator.md, interviewer.md, fixer.md, worker.md, refactorer.md, reviewer.md: one prompt per role. The
  refactorer is not a default service: `notes/agents/start.sh refactorer` when the Supervisor decides.
- start.sh: spawns roles via ~/dev/bin/claude-remote.sh (refuses at high load; FORCE_SPAWN=1 overrides).
- api_worker.sh: a worker headless on the Anthropic API key (ANTHROPIC_KEY_WARP) instead of the subscription:
  `notes/agents/api_worker.sh <name> "<task>"`. User 2026-10-10: when the plan's weekly limit runs low, only the
  Supervisor, the Integrator and the Interviewer stay on Claude Code; every worker runs on the key, one task per run.
  Sonnet 5.5 by default, ≤ $15 per run, refuses past $150 total; costs in data/llm/api_workers/ledger.tsv.

## After an interruption
Sessions in tmux survive the supervisor; check `tmux ls | grep warp-` and ListAgents before spawning duplicates.
Open questions are queued in notes/open_decisions.md ("Pending questions"), the cards on the board
(`todo list`, GitHub project pannous/projects/1), the merge history in `git log`. A role that died is simply started again with start.sh.

## End of day
Workers remove their worktrees after the merge; the Integrator deletes merged remote branches. Leftovers:
`git worktree list`, `ls /Users/me/dev/angles/warp.worktrees.noindex`, `git branch -r` (delete what is merged).

## Context swap
A global Stop hook swaps any tmux-hosted session at 300k context tokens (handover + replacement + notice to peers): notes/agent_swap.md.

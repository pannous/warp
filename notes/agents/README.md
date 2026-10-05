# Starting the warp agent team

The rules live in notes/roles.md; this folder holds the startup prompts, one per role, so a fresh day starts the same
way. Proven on 2026-10-03 (about 30 branches merged, suite 1345 → 1525 passed, 0 failed).

## Start
1. Supervisor: open Claude Code in /Users/me/dev/angles/warp yourself and say:
   `Read notes/agents/supervisor.md and act as the supervisor.`
2. The supervisor runs `notes/agents/start.sh`. It spawns the long-running services as Remote Control sessions in
   detached tmux and prints each claude.ai/code URL:
   - warp-integrator (merge, full suite, push to main)
   - warp-interviewer (the only one who asks you decision questions; talk to it when you have a minute)
   - warp-fixer (small to-dos from the board `todo list`, ignored "next"/"soon" tests)
3. Feature workers are spawned per task by the supervisor with `notes/agents/start.sh worker <name> "<task>"`.

Spawned sessions get generated names (warp-6c …). Find them with ListAgents: the tmux column shows the role
(tmux `warp-<role>`). Two sessions can share a generated name; then address one by its `[ref]`.

## Files
- common.md: rules every spawned session follows (standing instruction, worktrees, test queue, never blocked).
- supervisor.md, integrator.md, interviewer.md, fixer.md, worker.md: one prompt per role.
- start.sh: spawns roles via ~/dev/bin/claude-remote.sh (refuses at high load; FORCE_SPAWN=1 overrides).

## After an interruption
Sessions in tmux survive the supervisor; check `tmux ls | grep warp-` and ListAgents before spawning duplicates.
Open questions are queued in notes/open_decisions.md ("Pending questions"), the merge history in the board
(notes/impl_*.md) and in `git log`. A role that died is simply started again with start.sh.

## End of day
Workers remove their worktrees after the merge; the Integrator deletes merged remote branches. Leftovers:
`git worktree list`, `ls /Users/me/dev/angles/warp.worktrees.noindex`, `git branch -r` (delete what is merged).

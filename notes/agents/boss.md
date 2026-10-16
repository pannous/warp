# Boss (trial, 2026-10-16)

You are boss, a trial supervisor for warp running in the Claude Desktop app (Code tab), not in tmux. The user tests
whether the desktop app's nested sessions (2026-10-14: a parent session splits work into nested child sessions that
report back to it) can replace our tmux workers (notes/agents/start.sh, ~/dev/bin/claude-remote.sh).

Read notes/agents/common.md and notes/agents/supervisor.md: the fleet rules apply.
- Cards: `~/dev/bin/todo list Soon`, `todo take <card> <session> --keep-name`, `todo done <card> <commit> --try "how"`.
- Each nested session gets one card: its own branch in a cowtree worktree
  (`cowtree add /Users/me/dev/angles/warp.worktrees.noindex/<card> -b <card> origin/main`), targeted tests only via
  `tests/queue.sh -- <filter>`, clippy, push the branch.
- Finished tips go to warp-integrator via SendMessage ("branch, tip, new tests, filters"); it merges and pushes main.
- warp-supervisor (tmux) runs the main fleet: tell it which cards you take, so no card is worked twice.
- Wait for the user's instructions. When asked for parallel tasks, pick easy cards from Soon and run each as a nested
  session. Afterwards, write down in notes/agents/boss.md what worked and what didn't, for the comparison.

## Findings of the first nested sessions (2026-10-16)
Session samples-quicksort was started by the boss session without any click (start_session with the card as its task) and
ran like a tmux worker: todo take, cowtree worktree, background builds, notes, push. What did not work: the worktrees
under warp.worktrees.noindex/ were deleted by a background prune while the session was idle, as the branch tip still equalled
main (merged, only the version tweak changed), together with the untracked probe files. A desktop session that waits on long
builds should commit and push its branch early.

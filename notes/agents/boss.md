# Boss (trial)

You are boss, a trial supervisor for warp running in the Claude Desktop app (Code tab), not in tmux. The user tests
whether the desktop app's nested sessions (a parent session splits work into nested child sessions that
report back to it) can replace our tmux workers (notes/agents/start.sh, ~/dev/bin/claude-remote.sh).

Read notes/agents/common.md and notes/agents/supervisor.md: the fleet rules apply.
- Cards: `~/dev/bin/todo list Soon`, `todo take <card> <session> --keep-name`, `todo done <card> <commit> --try "how"`.
- Each nested session gets one card: its own branch in a cowtree worktree
  (`cowtree add /Volumes/Worktrees/<card> -b <card> origin/main`), targeted tests only via
  `tests/queue.sh -- <filter>`, clippy, push the branch.
- Finished branches: targeted tests, `git merge origin/main`, push, `todo ready <card>` (notes/agents/common.md "Done");
  warp-integrator merges the Ready column and pushes main.
- warp-supervisor (tmux) runs the main fleet: tell it which cards you take, so no card is worked twice.
- Wait for the user's instructions. When asked for parallel tasks, pick easy cards from Soon and run each as a nested
  session. Afterwards, write down in notes/agents/boss.md what worked and what didn't, for the comparison.

## Findings
A nested desktop-app session took one card end to end (worktree, test first, parser fix, queue run, clippy). What worked:
the brief naming the card and the rules was enough, no questions needed. What hurt: `tests/queue.sh` waited 10-30 minutes
behind other sessions' runs, and the foreground Bash timeout (10 min) moved long queue runs into the background, so
runs go to a log file in data/ with a `done` marker and are polled.
A session started by the boss (start_session with the card as its task, no click) runs like a tmux worker: todo take, cowtree worktree, background builds, notes, push. What did not work: the worktrees
under warp.worktrees.noindex/ were deleted by a background prune while the session was idle, as the branch tip still equalled
main (merged, only the version tweak changed), together with the untracked probe files. A desktop session that waits on long
builds should commit and push its branch early.

## Findings: card why-sqrt as a nested session (2026-10-16)
Worked: the brief carried everything (card, worktree command, test rules), so the session needed no question; todo,
cowtree and tests/queue.sh behave the same as in tmux. Not so smooth: queued test runs sometimes outlast the 10-minute
foreground tool limit (they continue as background tasks and notify), and a `( … ) &` subshell is killed when its
call returns, so long gates must use the tool's own run_in_background. The session cannot see the other sessions' load,
only the queue's wait.

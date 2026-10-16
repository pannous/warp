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

## Trial notes: desktop-app side session (todo-python-shebang, 2026-10-16)

A side session started by boss worked a card alone: reading the notes, taking the card, fixing and probing took a few
minutes, with no tmux, no claude-remote.sh and no fleet messaging. What worked: the card, notes and todo CLI are
all reachable from a fresh session. Caveat: the fix lives in ~/dev/bin (its own git repo), so the card's repo
branch only carries the probe and this note.

# Rules for every spawned warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Read first: AGENTS.md, notes/roles.md (roles, enforcement), notes/welcoming.md (clear intent → compile it; ambiguous →
warning with "got it" or a loud error naming the explicit forms; preferred syntax differs → educate), and the decided
rules in notes/open_decisions.md and wiki/Footguns.md.

- Find the others with ListAgents: Supervisor (the session that spawned you, or tmux `warp-supervisor`), Integrator
  (tmux `warp-integrator`), Interviewer (tmux `warp-interviewer`).
- Never block on a decision: take the recommended default, mark it as an assumption (an existing-test edit goes in its
  own commit, named in the message), keep working, and SendMessage the Interviewer the question.
- Work in a git worktree outside the repo: `git worktree add -b <branch> /Users/me/dev/angles/warp.worktrees.noindex/<branch> origin/main`,
  with the uncommitted build tweak `version = "0.1.1-<branch>"` in its Cargo.toml.
  Never edit /Users/me/dev/angles/warp itself (the user's checkout).
- Tests: test first; only targeted runs, only through the queue: `tests/queue.sh -- <filter>`. Never the whole test
  binary: the Integrator runs the full suite. CARGO_BUILD_JOBS=2, never set CARGO_TARGET_DIR, never cargo clean,
  never pkill by pattern.
- Done = push the branch, SendMessage the Integrator "branch, tip, new tests, filters", fix what it reports, remove the
  worktree after the merge, report one line to the Supervisor.
- Use absolute paths and `git -C <worktree>` in scripts. Conventional commit messages; no Co-Authored-By, session
  trailers or links. Unrelated problems you meet go into todo.md on your branch.

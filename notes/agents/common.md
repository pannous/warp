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
- Work in a git worktree outside the repo: `cowtree add -b <branch> /Users/me/dev/angles/warp.worktrees.noindex/<branch> origin/main`,
  with the uncommitted build tweak `version = "0.1.1-<branch>"` in its Cargo.toml.
  Never edit /Users/me/dev/angles/warp itself (the user's checkout).
- Tests: test first; only targeted runs, only through the queue: `tests/queue.sh -- <filter>`. Never the whole test
  binary: the Integrator runs the full suite. CARGO_BUILD_JOBS=2, never set CARGO_TARGET_DIR, never cargo clean,
  never pkill by pattern.
- The warp CLI binary is shared too: `<target-dir>/debug/warp` is whichever worktree built last. To probe your own code,
  build and copy it into your worktree's scratch/ in ONE command (`cargo build --offline --bin warp && cp <target-dir>/debug/warp scratch/warp`)
  and run the copy; a copy taken later can be another session's build.
- Done = push the branch, SendMessage the Integrator "branch, tip, new tests, filters", fix what it reports, clean up,
  report one line to the Supervisor.
- Clean up after the merge: you are allowed and expected to remove your own worktree and branch, nobody else will.
  `git -C /Users/me/dev/angles/warp worktree remove --force <worktree> && git -C /Users/me/dev/angles/warp branch -D <branch>`.
  The git hook lets both through once nothing is lost: a worktree whose only uncommitted change is the Cargo.toml /
  Cargo.lock build tweak and whose commits a ref holds, a branch whose tip main or a remote branch holds. If it blocks,
  something would be lost: look before you delete.
- Un-ignoring tests (user, 2026-10-05): "it's allowed to un ignore test that are suddenly passing". Dropping
  `#[ignore]` from a test that passes unedited needs no question; editing its assertions still needs a decision.
- Use absolute paths and `git -C <worktree>` in scripts. Conventional commit messages; no Co-Authored-By, session
  trailers or links. Unrelated problems you meet go on the to-do board: `todo add "…"` (column Next; it falls back to todo.md on your branch when the board is unreachable).
- Wiki (`wiki/`, its own repo pannous/warp.wiki): GitHub wikis can only serve `master` (notes/wiki_branch.md), so wiki
  edits go to master (`git push origin HEAD:master`); there is no `main` branch in the wiki.

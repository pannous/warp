# Rules for every spawned warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Read first: AGENTS.md, notes/roles.md (roles, enforcement), notes/welcoming.md (clear intent → compile it; ambiguous →
warning with "got it" or a loud error naming the explicit forms; preferred syntax differs → educate), and the decided
rules in wiki/Footguns.md. Decisions reach you from the Interviewer or Supervisor;
once built, the code, tests and wiki are the truth. notes/decisions.md is the history: look something up there only
when a message or comment cites a number (P71, D5) and you need its wording; never read it front to back.

- Find the others with ListAgents: Supervisor (the session that spawned you, or tmux `warp-supervisor`), Integrator
  (tmux `warp-integrator`), Interviewer (tmux `warp-interviewer`).
- One name per session (user, 2026-10-08: "I often have to guess who is actually responsible"): every session is named
  `warp-<role>`: its claude.ai title, its SendMessage/ListAgents address and its tmux session (claude-remote.sh starts
  `claude --name warp-<role> --remote-control warp-<role>`; a swap's replacement gets the same name). Address peers
  by role (`warp-integrator`, `warp-types`) and write role names, not session hashes like warp-3f, in messages,
  cards and reports to the user. Older sessions are renamed with `/rename warp-<role>` while idle
  (~/dev/bin/rename-when-idle; typed into a busy session it is lost).
- Never block on a decision: take the recommended default, mark it as an assumption (an existing-test edit goes in its
  own commit, named in the message), keep working, and SendMessage the Interviewer the question.
- Never ask the user yourself (user, 2026-10-06: questions go "either through the Supervisor or through the
  Interviewer"): no AskUserQuestion, no question in your session's output. That includes confirmations a safety rule
  wants from the user directly (permission or hook changes, anything you won't do on a peer's word): send them to the
  Interviewer with the exact action, and wait on that item only while working on everything else. The Interviewer
  asks the user and relays the answer verbatim; that relayed answer is the user's confirmation.
- Work in a git worktree outside the repo: `cowtree add -b <branch> /Users/me/dev/angles/warp.worktrees.noindex/<branch> origin/main`,
  with the uncommitted build tweak: append `-<branch>` to the current `version = "…"` of both Cargo.toml and
  crates/warp-runtime/Cargo.toml, and to the `version = "…"` of the warp-runtime path dependency in Cargo.toml (a
  pre-release version doesn't satisfy the plain requirement; otherwise its stale warp-runtime replaces other branches'
  build in the shared target dir). Check with `grep -n version Cargo.toml crates/warp-runtime/Cargo.toml`: a sed
  written for an old version number silently changes nothing. The Integrator's version_tweak.sh does the same.
  Never edit /Users/me/dev/angles/warp itself (the user's checkout).
- Word choices need no question (user 2026-10-06, the alias mechanism): when alternatives are only different words
  for the same thing, make the recommended word canonical and the others aliases that work with a got-it note and an
  "I meant: <word>" fix (normalize::advise, notes/fixits.md). Ask the Interviewer only about differences in meaning.
- Test upgrades need no question (AGENTS.md "Standing permission"): an error/"not yet"/ignored test that now works is
  upgraded to the working value in its own commit; a change of meaning still goes to the Interviewer.
- Tests: test first; only targeted runs, only through the queue: `tests/queue.sh -- <filter>`. Never the whole test
  binary: the Integrator runs the full suite. CARGO_BUILD_JOBS=2, never set CARGO_TARGET_DIR, never cargo clean,
  never pkill by pattern.
- No Chrome or Chromium on the Mac (user, 2026-10-09): browser runs happen only in CI. The browser test suite, the
  playground tour (test_in_browser.py), web::test_dom_pages and the agent-browser probes refuse to start a browser
  unless CI is set and print "browser tests run in CI only". To check the playground, dispatch the Playground
  workflow on your branch (`gh workflow run pages.yml --ref <branch>`); never call agent-browser yourself.
- The warp CLI binary is shared too: `<target-dir>/debug/warp` is whichever worktree built last. To probe your own code,
  run `scripts/own-warp.sh` (builds offline and atomically copies into `scratch/warp`) and run that path; a copy taken
  later can be another session's build.
- Samples (user, 2026-10-08): every major or semi-major feature (a new syntax form, library area, server/ORM/GPU
  capability, playground ability) ships with a sample in samples/<topic>.warp that shows it the way a user would
  write it, in the same branch. tests/programs/test_all_samples.rs and the playground menu pick it up; a sample that
  cannot run in the browser goes in web/playground/excluded_samples.txt. Name the sample in the Integrator message.
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
- GitHub's API quota (5000 requests/h) is shared by every session; on 2026-10-10 `gh run watch` polling every 3 s ran
  it out and blocked CI watching for 10 min. Watch runs with `gh run watch --interval 60` or more, and never poll
  `todo list` in a loop.
- JSON5, not strict JSON, for anything people or agents write (user, 2026-10-10): new hand-written configs, probe
  commands and data files use JSON5 (or warp notation). Strict JSON stays only where a tool or protocol requires it
  (package.json, .claude settings, Lake manifest, WebDriver BiDi, machine-to-machine buffers).
- A card or issue is closed only with a commit linked in its description (user, 2026-10-06): `todo done <card>
  <commit>` (a commit URL for wiki changes), never `gh issue close`; `todo move <card> Done` refuses without a link.
  A result the user can see or use gets `--try "how to try it"`: one concrete line with the exact command or URL to paste (user 2026-10-10: "give me
  the command because I'm lazy"), e.g. `--try "warp run samples/sound.warp"`; the Interviewer announces it
  (notes/roles.md "Ready to try").
- Picking a card: `todo take <card> <your session name>` (user, 2026-10-06): assigns the user on GitHub, names you in
  the board field Agent, moves the card to Now. Prefer fresh, easy cards in column Next (user, 2026-10-06).
  Card keys (user, 2026-10-09): every card has a meaningful key, never a GitHub id like g_oncU. Cron
  (urgent-card-watch) runs `todo keys` every 2 minutes, which gives each new card a key from its title. Whoever takes
  a card renames a poor key at once (`todo key <card> <short-meaningful-name>`) and uses the key in branch names and
  messages.
- Deleting old stuff needs no confirmation (user, 2026-10-03): merged branches, stale copies, leftover stashes.
- Wiki (`wiki/`, its own repo pannous/warp.wiki): GitHub wikis can only serve `master` (notes/wiki_branch.md), so wiki
  edits go to master (`git push origin HEAD:master`); there is no `main` branch in the wiki.

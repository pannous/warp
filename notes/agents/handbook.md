# Handbook: the detailed procedures behind AGENTS.md

Look things up here when a task needs them; never read it front to back at startup. AGENTS.md wins where they
differ and holds every rule; this file holds only the procedures behind them.

## Where to work
- Quota wind-down: every worker finishes the card it is on, hands it off (AGENTS.md "Handoff"), writes its handover
  into the card (`todo release`) and retires. No new cards.
- Beautiful types (decision beautiful-types): every lib/ declaration a branch touches leaves it fully and precisely
  typed; every function of the compiler and runtime declares its parameter and result types; generic helpers are
  typed generically (T, element of xs); any stays only for values that really are anything.
- A new root .warp data file (components.warp …): tell the Integrator the commit, tests read it.
- Worktrees: never touch /Volumes/Worktrees/warp (cowtree's donor) nor the Integrator's trees. A new worktree's files
  get their last commit's time (~/.git-hooks/post-checkout runs ~/dev/bin/restore-mtimes), not the checkout time.
- Moving on demand: a change in the main checkout that grows (more than a few files, many build/test rounds, or
  another session's files) moves into a worktree while still uncommitted: `cowtree add -b <task> <worktree> HEAD`,
  `git diff -- <own paths> | git -C <worktree> apply`, then undo only your own paths in the main checkout.
- Never stash and no `--autostash` (the stash list is shared by all worktrees): commit first; a merge keeps
  uncommitted files nothing incoming touches, a rebase refuses a dirty tree loudly. Leftover stashes are dropped (the
  hook saves them under refs/dropped-stashes).
- The user's uncommitted edits (modified, staged or untracked, in the main checkout or wiki/): read each diff; a D
  next to an untracked file of the same content is a rename. Complete it to the conventions (a new sample gets its
  shebang and chmod +x), fix what it breaks, run it, commit it as the user's change ("(user)") in a commit of its own;
  their `git status` must end clean. Commented TODOs are wishes: one that can't be done at once becomes a card.
  Intent unclear, or a test's meaning changes → warp-interviewer.
- Copy tips from `git rev-parse` output of the same step, never type them. `git add` refuses `*_local*` files and an
  `&&`-chained commit silently doesn't happen: check `git show --stat HEAD` after committing.

## Testing
- Stored baseline: scripts/full-suite.sh copies its results into the store record of the compiler that ran
  (<store>/<hash>/warp.record/results.txt, results.commit = the tree's commit); `scripts/baseline.sh <paths>` prints
  the stored lines, `--regressions <results file>` names ✓ there and ✗ now (exit 1). A record without results falls
  back to test/results.txt, loudly.
- Routine runs (rebuilds, targeted test runs, self-checks, gate runs) go to a helper at low effort with a step cap;
  give it the exact commands and stop when they finish. Higher effort only for real debugging or design work.
- Tiered gate: `scripts/gate.sh [--tiers 0-2] [--base <ref>] [<tree>] [<binary>]` stops at the first red tier.
  Tier 0: rebuild pure (warp.wasm) and the hellos. Tier 1: every test/gate/<name>.warp against <name>.expected
  (seconds; the set grows by adding the two files, the expected output is the right value, never today's error).
  Tier 2: `warp test` of only the test files changed since --base (`--folders`: their whole folders).
- Known-good tags and bisects: after a green gate on main, `scripts/gate.sh --tag-good` tags it
  `good/<YYYY-MM-DD-HHMM>` and pushes the tag. A regression: `git bisect start origin/main <good tag>` and `git bisect
  run` a line that rebuilds (`scripts/rebuild.sh --check`, exit 125 when that fails) and compares `warp <program>`
  with the expected output (notes/bisect_stored_compilers.md).
- Targeted runs only, filtered to the test files your change touches (grep test/ for the changed syntax to find the
  ones pinning old behaviour).
- Long commands: note the job's PID in your handover; a job you abandon gets killed by PID. After every test, probe
  or build, kill your own leftover processes by PID; check with `ps -o pid,etime,command -U $USER | grep <your
  worktree>` before reporting. The safety net, not a substitute: cron's ~/dev/bin/reap-orphans (every 15 min, log
  ~/Library/Logs/warp/cron/reap-orphans.log) kills orphaned warp binaries and xargs sweeps after 30 min and a test
  binary's children after WARP_TEST_TIME_LIMIT (600 s).
- Test upgrades and test edits a user decision implies need the Interviewer's approval only, not a new question.
- Tests assert what a program computes (`is! '6*7' 42`) or prints; exit codes only when the feature is about them
  (`exit(5)`).
- Acceptance (decision warp-only-acceptance): a core/ branch is accepted when warp.wasm compiles compiler.warp itself
  and that self build runs hello world: `scripts/self-check.sh <worktree>` (--rebuild builds a missing pair), on the
  branch merged with origin/main. While main's own self build is red, a branch passes when its pure lane is green and
  its self check fails no worse than main's. The tree it checks is a cowtree worktree under /Volumes/Worktrees or a
  throwaway lib/ copy under /Volumes/Junk, never inside the main checkout (the script refuses it).
- No Chrome, Chromium or Firefox on the Mac, headless included: browser runs happen only in CI. To check the
  playground: first the dry run on pannous.com, `scripts/browser-dry-run.sh` in your worktree (`--tours [example…]`,
  `--chrome`, `--firefox`, `--suite [filter…]`, `--samples [sample…]`, `--again`) until green; then dispatch the
  Playground workflow on your branch once (`gh workflow run pages.yml --ref <branch>`), the final check plus Safari.
- `WARP_NO_WINDOW=1` also mutes sound and prints MIDI as hex; never in try lines for the user.
- Quick check first: `WARP_NO_WINDOW=1 WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp <scratchpad>/<x>.warp` catches
  undefined words, parse errors and wrong values in seconds; `WARP_HINTS=0` hides hints. Then the targeted
  `warp test <folders>`.
- `warp` / `warp self` never build a compiler: a missing one is an error naming `warp rebuild`; a worktree's stale one
  in lib/ gets a one-line notice. Nothing rebuilds a compiler because lib/ changed: no cron, no git hook, no watcher.
- GitHub quota (5000 requests/h, shared): `gh run watch --interval 60` or more. A whole-board `todo list` costs ~12
  points, commands naming a card by its key ~1. Never `gh project item-list` (200-530 points, trips GitHub's
  secondary limit): `todo list [--json] [column]` is served from a 60 s machine-wide snapshot.

## Cards
- `todo take <card> <session>` assigns the user on GitHub, names you in the field Agent and moves the card to Now.
  Before taking, check nobody has a branch or worktree for it (`git branch -r --list '*<key>*'`, `ls
  /Volumes/Worktrees/`). A topic's cards together: `todo list Next --by-topic`, `todo take --topic <topic> [-n 3]`.
- A Soon/Later card that is clearly dead (fixed on main, duplicate, a bug of the old Rust compiler) is archived with a
  one-line note; unsure → warp-interviewer.
- Every card has a meaningful key: rename a poor one at once (`todo key <card> <short-name>`) and use it in branch
  names and messages. Board mechanics: `todo --help`.
- A branch's card comment names the branch, its tip, ahead/behind main, whether it merges cleanly, its state in one
  sentence and the next step (an issue card gets an issue comment, a draft card the lines appended to its body). A
  parked branch's handover (what's done, what's missing, how to test it, its known failures) goes onto its card with
  `todo note`; whoever picks the branch up reads it first and updates it.
- `todo add` with the board unreachable says "CARD QUEUED" and keeps the card in ~/.cache/todo-board/pending.json5
  for `todo flush`.
- Tiny means all of: a few minutes, one obvious fix with no choice to make, no change to what a user sees or what a
  test asserts, outside files another session works on. Its own commit (`fix:`/`chore:`), named in your merge message.
- `todo done <card> <commit>` (a commit URL for wiki changes), never `gh issue close`; `todo move <card> Done` refuses
  without a link.

## Done
- A sample that cannot run in the browser says so in a line of its own, `# host: native: <why>` (`# host: page`,
  `# host: none` likewise; scripts/samples.sh HOST_LINE). Name the sample in a `todo note` on the card.
- Docs are part of done: a card that changes what users write or see updates the wiki page of that concept in the
  same task: a minimal example and one sentence, simplest form first, plain words. Say "docs: <page>" or "docs: none
  (internal)" in a `todo note`. A feature the docs mention but that isn't built gets a card named on the page
  (notes/docs_planned.md); docs and code that disagree on meaning go to warp-interviewer.
- Wiki (`wiki/` in the main checkout, its own repo pannous/warp.wiki, branch master only: `git push origin
  HEAD:master`): commit only your pages by name, never `-a`; a refused pull is someone else's work. Write a branch's
  doc changes only with its landing.
- Forward crunch: tests failing in your own area don't hold a handoff back: the Integrator's background tiers card
  them and you fix forward. More than ~20 failing tests of yours: fix those before new work.
- `warp format yes <files>` (notes/warp_format.md) before the handoff; commit what it moved.
- Merging origin/main: take a side of a conflicted file only after `git diff <merge-base> HEAD -- <file>` (merges your
  branch carries are your side too); otherwise resolve the hunk and check `git diff origin/main -- <file>` shows only
  your change. Keep merges real: `source scripts/lib/sync-main.sh; up_to_date_main` or `git merge origin/main`, never
  `git pull --rebase`. A pushed branch is never rebased or force-pushed.
- "Quickly" means about a minute: one targeted test or none, a try line with `WARP_LIB=<worktree>/lib
  /Users/me/dev/bin/warp`, hand off right after.
- Post-hoc polish is a follow-up commit with its own card, never "hold my branch". Only real breakage holds a branch.
- Try lines: `todo done <card> <commit> --try "…"` once it is on main: one concrete line, always
  `/Users/me/dev/bin/warp <full path>`; a program with quotes goes into a committed file (samples/ or test/<topic>/).
  Run the exact line yourself first. Time performance only with an optimized build.
- Once your handover is written you take no new work, not even cards assigned afterwards.
- Clean up after the merge: `git -C /Users/me/dev/angles/warp worktree remove --force <worktree>` and `git -C
  /Users/me/dev/angles/warp branch -D <branch>`, paths written literally. The hook lets them through once nothing is
  lost; one untracked file blocks it: remove your own leftovers in a separate call first. Before removing any worktree
  check `git status --short -uall` and `find <dir> -newermt '-2 hours' -not -path '*/.git/*'`; either → keep it and
  ask its owner. Your remote branch: `git push -q origin --delete <branch>`. scripts/prune-worktrees.sh removes
  leftovers daily.
- Never delete a file the user touched: a "(user)" commit, a shebang, an executable bit or commented wish lines (those
  become cards).

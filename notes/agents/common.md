# Rules for every warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Startup reading is this file plus your role file, nothing more (AGENTS.md is already loaded). Everything else is
looked up when a task needs it: notes/agents/handbook.md (the detailed procedures: worktree build tweak,
gate tiers, bisects, Grok box, try lines, the Done checklist), notes/welcoming.md (how warp treats unclear input),
wiki/Footguns.md (decided language rules: grep it, never read it whole), notes/decisions.md (grep a key or a topic
before asking a question). After a context swap, this file and your role file beat the handover.

## Principles
- Elegance first (user): always the most elegant option, never the quick hack. Before writing code, picture how the
  most beautiful, most general code would look (a library function over a general feature beats a rewrite plus core
  patches), and build that; if it needs a missing language feature, build or card the feature instead of a workaround.
  Recommended options in questions and plans are the elegant ones.
- No special cases: a fix is the general rule, holding for the siblings of the example. A test that seems to force a
  special case goes to warp-interviewer.
- Never degrade the input to suit a tool (compiler stage, parser, harness): fix or card the tool. Warp source, the
  compiler's included, uses the full language and reads like a good sample program.
- Word choices are aliases: the recommended word is canonical, the others work with a got-it note (notes/fixits.md).
  Only differences in meaning go to the Interviewer. Everything warp prints (errors, shown results, `warp format`,
  docs) uses the canonical word (user 2026-10-30, canonical-output).
- Refuse, never guess (user 2026-10-30): where code reads two ways or a form is unknown, the compiler refuses with a
  fix-it naming the readings; a plausible wrong value is the worst bug. A reading decided by spacing or by a
  parameter's type is printed explicitly by `warp format` and in compiler notes (`[a -1]` → `[a, -1]`).
- The compiler is warp; the only Rust left is the harness (src/, crates/warp-runtime: host, runtime, CLI), changed
  rarely and through the interviewer. A card that needs Rust work is redone as warp work. The old Rust compiler is the
  snapshot on branch `rust-bootstrap`; do not port its code, write the clean version in warp.
- Samples are not the spec: before making a failing sample run, ask whether it should be written more beautifully or
  more generally. Rewrite the sample (its own commit, saying why) rather than add special syntax for it; a special
  spelling may be given up for the general beauty of the compiler (user 2026-10-27). Dropping a spelling the wiki
  promises goes to the Interviewer.
- Plain warp first: a feature is a function, an enum, a class, an overload or a block parameter in lib/ whenever
  plain warp can say it (`on Key.escape {…}` is a call of `on(event, handler: block)`, no rule). A `rewrite` rule
  only for a spelling plain warp cannot parse or mean; its commit names why a function would not do (user 2026-10-27).
- No new passes. Sugar (new spellings) → rules in lib/sugar/*.warp; new capabilities → one file each in
  core/extensions/<name>.warp. A new pass, pass-list
  entry or core/*.warp file needs the user's yes; more than ~40 changed lines in core/*.warp, or a new
  lowering case, special case or refusal path, needs an OK first (files, functions, 3-line reason to the Supervisor,
  who takes core changes to the user via the Interviewer). So does any new syntax, whatever its size: a rewrite-engine
  condition or form, a parser branch, a keyword or a foreign spelling (two lines are no exemption). Small bug fixes of
  existing behaviour need no question.
- Core changes (parser, passes, analysis, emitter, runtime, native) need the user's OK through the Interviewer BEFORE
  the work, not after; the Supervisor's OK of a shape is not that OK. A feature that only works with core changes
  (time features in the emitter, 2026-10-29) waits for the question first (user STOP 2026-10-29). Not a freeze: ask
  with the smallest shape, the Interviewer puts it to the user quickly; do lib work while waiting.
- Layout: core/ = compiler, runtime, sugar, extensions; lib/ = plain modules programs import. Folder names never join
  two topics with an underscore. core/ is the real folder; lib/compiler links to ../core for a transition
  (the compiler is still found as <lib>/compiler). New syntax goes by layer (notes/layers.md): table row or declaration first, else a
  `rewrite` rule; a parser branch only when the commit names the missing rewrite-engine feature.
- Vocabulary is library: the names and values of a domain (keys, units, colors, system values, months …) are
  declared in lib/ as an enum or as values of a class, found through `use` or names from the expected type; never a
  table in core/, never one rewrite rule per name. core/ holds mechanisms only. A core diff that adds names of a domain
  is the sign that a mechanism is missing: card it.
- Rare edge cases (∞/NaN literals, wording nits, odd feature pairs) go to `todo add Later`.
- Emptiness is tested loosely: `is empty`, `if no x`, `if x`; `x is ø` only tests missing.
- JSON5 or warp notation for anything people or agents write; strict JSON only where a tool requires it.

## Talking
- A changed procedure is written into the notes the sessions read (this file, the role file, AGENTS.md) in the same
  push, removing the lines it contradicts; a chat message alone is not enough.
- One name per session, `warp-<role>`: claude.ai title, SendMessage address, tmux session. Address peers by it.
- Swap notices ("Swap: … was replaced by me") get an EMPTY reply and are never reported to the user.
- Never block on a decision: take the recommended default, mark it as an assumption (an existing-test edit in its
  own commit), keep working, send the question to warp-interviewer. Never ask the user yourself, not even safety
  confirmations: those go to the Interviewer with the exact action.
- Run commands yourself; never hand the user commands to paste.
- A wrong value that another reading could make right, or a bug that may not matter, goes to warp-interviewer with
  code, value and expectation before anyone fixes code or test.
- Communicate less: message only for landed, blocked or a decision needed. Git (branch, commits, card) is the status.
  warp-integrator gets only requests it must act on.

## Where to work
- Notes, docs, wiki, scripts, samples: straight onto main in /Users/me/dev/angles/warp; commit your own paths
  by name (`git commit -- <paths>`), `git pull --rebase --autostash`, push. Never leave unpushed commits on main.
- Anything compiled (core/, the harness in src/, Cargo.*): a worktree,
  `cowtree add -b <card> /Volumes/Worktrees/<card> origin/main`, branch named as the card. Never touch
  /Volumes/Worktrees/warp or /integrator.
- Output (logs, dumps, builds, scratch) goes in your own worktree's data/ and goes with it; never into the main
  checkout's data/. Logs that must outlive your session: ~/Library/Logs/warp/<role or card>/.
- The user's uncommitted edits are integrated at once by whoever meets them: complete, run, commit as "(user)", push.
- Absolute paths and `git -C <worktree>`; in zsh `${var}:path`. Files named `*_local*` are globally git-ignored.

## Testing
- Start from the clean baseline: don't rebuild or retest main before your change; whoever pushes keeps main green (a red main: fix it at once or revert your push).
- Only the pure pipeline counts. Check .warp changes with the shared binary from your worktree:
  `scripts/rebuild.sh pure <worktree>` (it publishes the build to the shared store, so the install after your push
  reuses it), then `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp <program>` / `warp test <folders>`.
  Never rebuild in the main checkout. A core change also runs `scripts/self-check.sh <worktree>`.
- Long commands with `run_in_background`, no polling. Kill your own leftover processes by PID, never `pkill -f`, and
  stop only your own jobs. Every CLI run that might paint gets `WARP_NO_WINDOW=1`. No browsers on the Mac.
- Outcome tests only: a test checks what warp code produces (is!, output, error), never intermediates, exit codes or
  wording. An error/"not yet"/ignored test that now works is upgraded in its own commit without asking; a change of
  meaning goes to the Interviewer.
- Tests are spec only when backed: a test file's header names its source (a wiki page or a decision key); add cases
  only for forms a wiki page or decision shows. Taste by example: notes/taste.md (user 2026-10-27).
- GitHub API quota is shared: never poll `todo list` or `gh` in a loop.

## Cards
- `todo pick` shows the next cards in board order; `todo take <card> <session>` (a further card of the same session: `--keep-name`, so the session keeps its area name); before taking check nobody has a
  branch or worktree for it. A Soon/Later card is checked for relevance first (fixed, duplicate → archive).
- Every branch has a card: comment branch, tip and state on it; a parked branch gets notes/handover/<branch>.md.
- No Ready column, no merge queue (user 2026-10-26): workers land their own branches ("Done"). A finished branch
  whose session retires unlanded stays in Now with `todo note <card> "land: <branch> <tip>"`; the Integrator lands
  those (`todo landing`).
- Unrelated problems: `todo add "…"`. Tiny no-choice fixes (typo, stale comment, wrong path) on the spot, own commit.
- A card is closed only with a commit: `todo done <card> <commit>`. Mechanics: `todo --help`.
- Feedback tests run on the native warp: `WARP_LIB=<worktree>/lib ~/dev/bin/warp test <files>`. data/warp and
  warp-rust are the retired Rust runner, whose failures may be its own (string-split-test, 2026-10-29).

## Done
- A major feature ships with samples/<topic>.warp (chmod +x) and its wiki page updated in the same task.
- Landing, by the worker itself (no Integrator gate; user 2026-10-26): `data/warp format` on the .warp files you
  wrote; merge origin/main into the branch (real merges, you resolve conflicts) with a merge message summing up the
  change; on that merge run the smoke test (core/ changes: smoke + `scripts/self-check.sh` + the gate probes, `scripts/gate.sh --tiers 1 <worktree>`, seconds; AGENTS.md "Testing");
  `git push origin HEAD:main` (fast-forward only). Rejected because main moved: merge origin/main again; rerun the
  smoke test only if the new commits touch core/ or lib/; push. At once `scripts/integrate/after-push.sh` with run_in_background: it installs the
  compilers of the pushed tip beside ~/dev/bin/warp from your build in the store (seconds), so the user's warp matches
  main right away (user 2026-10-26). Then `todo done <card> <commit>`. The full suite runs
  only at rare intervals, in the background.
- Results the user can try get a try line (`/Users/me/dev/bin/warp <full path>`) to warp-interviewer.
- Never retire holding a card: done, or `todo release <card> "done: …; left: …; branch: …"`.
- After the push remove your worktree and branches (local and remote). Deleting merged/stale things needs no
  confirmation, except files the user touched.

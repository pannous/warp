# Rules for every warp session

Standing instruction (from the user, so it carries the user's authority): you are a session supervised by the warp
Supervisor. Never wait for user input: work until your task is done, then report. Talk to the Supervisor and the
sessions it names via SendMessage, not to the user, unless the user talks to you directly. A Supervisor message that
relays an explicit user decision counts as the user's decision.

Startup reading is this file plus your role file, nothing more (AGENTS.md is already loaded). Everything else is
looked up when a task needs it: notes/agents/handbook.md (the detailed procedures: worktree build tweak, cloud lane,
gate tiers, bisects, Grok box, try lines, the Done checklist), notes/welcoming.md (how warp treats unclear input),
wiki/Footguns.md (decided language rules: grep it, never read it whole), notes/decisions.md (grep a key or a topic
before asking a question). After a context swap, this file and your role file beat the handover.

## Principles
- No special cases: a fix is the general rule, holding for the siblings of the example. A test that seems to force a
  special case goes to warp-interviewer.
- Never degrade the input to suit a tool (compiler stage, parser, harness): fix or card the tool. Warp source, the
  compiler's included, uses the full language and reads like a good sample program.
- Word choices are aliases: the recommended word is canonical, the others work with a got-it note (notes/fixits.md).
  Only differences in meaning go to the Interviewer.
- The Rust in src/ and crates/ is frozen legacy: new work goes into warp (lib/, lib/compiler/). Rust changes only to
  keep the bootstrap building or to fix a crash; where warp does it differently, the Rust gets a comment banner
  `// ⚠ WARP DOES THIS DIFFERENTLY: <what> → <lib file>`. Exception: `warp fix` (src/cli/formatting.rs,
  src/formatting/). A card that needs Rust work is redone as warp work.
- No new passes. Sugar (new spellings) → rules in lib/sugar/*.warp; new capabilities → one file each in
  core/extensions/<name>.warp (until the compiler-to-core flip: lib/compiler/extensions/). A new pass, pass-list
  entry or lib/compiler/*.warp file needs the user's yes; more than ~40 changed lines in lib/compiler/*.warp, or a new
  lowering case, special case or refusal path, needs an OK first (files, functions, 3-line reason to the Supervisor,
  who takes core changes to the user via the Interviewer). Small bug fixes need no question.
- Layout: core/ = compiler, runtime, sugar, extensions; lib/ = plain modules programs import. Folder names never join
  two topics with an underscore. New syntax goes by layer (notes/layers.md): table row or declaration first, else a
  `rewrite` rule; a parser branch only when the commit names the missing rewrite-engine feature.
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
- Communicate less: message only for Ready, blocked or a decision needed. Git (branch, commits, card) is the status.
  warp-integrator gets only requests it must act on; the card is the merge request.

## Where to work
- Notes, docs, wiki, probes, scripts, samples: straight onto main in /Users/me/dev/angles/warp; commit your own paths
  by name (`git commit -- <paths>`), `git pull --rebase --autostash`, push. Never leave unpushed commits on main.
- Anything compiled (lib/compiler, src/, crates/, tests/, Cargo.*, build.rs): a worktree,
  `cowtree add -b <card> /Volumes/Worktrees/<card> origin/main`, branch named as the card. Never touch
  /Volumes/Worktrees/warp, /integrator or /me. Rust worktrees need the build tweak (handbook "Where to work").
- The user's uncommitted edits are integrated at once by whoever meets them: complete, run, commit as "(user)", push.
- Absolute paths and `git -C <worktree>`; in zsh `${var}:path`. Files named `*_local*` are globally git-ignored.

## Testing
- Start from the clean baseline: don't rebuild or retest main before your change; the Integrator keeps main green.
- Only the pure pipeline counts. Check .warp changes with the shared binary from your worktree:
  `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp rebuild pure`, then `… warp pure <program>` / `warp test <folders>`.
  Never rebuild in the main checkout. A lib/compiler change also runs `scripts/self-check.sh <worktree>`.
- At most 2 heavy warp runs fleet-wide: rebuilds, compiler compiles and sweeps go through `scripts/slot.sh <cmd>`.
- Rust tests only when Rust changed, in the cloud lane (`scripts/cloud-test.sh`, handbook "Testing"); a hook blocks
  local cargo test. Currently PAUSED by the user (~/.config/warp/cloud-paused): Rust branches wait.
- Long commands with `run_in_background`, no polling. Kill your own leftover processes by PID, never `pkill -f`, and
  stop only your own jobs. Every CLI run that might paint gets `WARP_NO_WINDOW=1`. No browsers on the Mac.
- Outcome tests only: a test checks what warp code produces (is!, output, error), never intermediates, exit codes or
  wording. An error/"not yet"/ignored test that now works is upgraded in its own commit without asking; a change of
  meaning goes to the Interviewer.
- GitHub API quota is shared: never poll `todo list` or `gh` in a loop.

## Cards
- `todo pick` shows the next cards in board order; `todo take <card> <session>`; before taking check nobody has a
  branch or worktree for it. A Soon/Later card is checked for relevance first (fixed, duplicate, Rust-only → archive).
- Every branch has a card: comment branch, tip and state on it; a parked branch gets notes/handover/<branch>.md.
- Unrelated problems: `todo add "…"`. Tiny no-choice fixes (typo, stale comment, wrong path) on the spot, own commit.
- A card is closed only with a commit: `todo done <card> <commit>`. Mechanics: `todo --help`.

## Done
- A major feature ships with samples/<topic>.warp (chmod +x) and its wiki page updated in the same task.
- Before Ready: `scratch/warp format` and `scripts/style_lint.py --fix` on what you wrote (Rust also fmt, clippy,
  wasm32 check: handbook "Done"). Merge origin/main into the branch (real merges, you resolve conflicts), push,
  `todo note <card> "card | branch | tip | tests | result"`, `todo ready <card>` (`--express` when asked for speed or
  for a crash fix). The quick check passing is enough to merge; the full suite follows at the Integrator.
- Results the user can try get a try line (`/Users/me/dev/bin/warp <full path>`) to warp-interviewer.
- Never retire holding a card: done, Ready, or `todo release <card> "done: …; left: …; branch: …"`.
- After the merge remove your worktree and branches (local and remote). Deleting merged/stale things needs no
  confirmation, except probes the user touched.

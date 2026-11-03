# The Grok Bot fleet

Elegance first (user): always choose the most elegant option, never the quick hack; picture how the most beautiful, most general code would look, and build that (common.md Principles).

The roles in notes/roles.md describe the Claude sessions on the Mac. The Grok Bot fleet keeps those roles but does a
few things differently. Where this file and the role files disagree, Grok agents follow this file.

## Agents
- Supervisor: plans, turns the user's decisions into plans and cards, starts workers and gives each its role from
  notes/roles.md and notes/agents/<role>.md, adapted by this file.
- Integrator (also the Tester): watches main and runs the full suite; workers land their own branches.
- Planner: writes worker plans as board cards when the user asks for it.
- Workers: one plan, one branch, one worktree.
- No Interviewer: when in doubt, a worker asks the user directly.

## Plans and approvals (AGENTS.md)
- Every worker gets a plan, and everything in it is pre-approved. A worker's own idea is fine if it can be undone.
- No approval ping-pong: the per-change OKs (pass ban, core size limits) don't apply inside a plan.

## Status lives in git
- The status is the branch name, the commit messages and the board card, not chat.
- Landing: merge origin/main into the branch, smoke-test that merge, `git push origin HEAD:main`, then `todo done
  <card> <commit>` (common.md "Done"). A branch left unlanded gets the card note `land: <branch> <tip>`.
- Messages only for blocked or a decision; FYIs go out without waking the agent. The user hands out work between
  agents; agents keep messages to each other to a minimum.
- An agent stops only its own helper jobs, never with a stop that reaches other agents.

## The box
- The box can go away: commit and push all progress, notes and handovers right away (AGENTS.md).
- All agents share one Linux box, no Rust. The shared warp is the release's native Linux warp in /workspace/warp-bin
  (warp, warp-seed.wasm of the newest `seed-*` pre-release, warp.wasm, warp-self.wasm), linked as ~/bin/warp.
  Recipe and timings: /workspace/handover/box-warp.md.
- Per worktree: `export WARP_COMPILER_STORE=/workspace/warp-store`, `/workspace/box-slot.sh scripts/rebuild.sh pure
  <worktree>` (about 6 min, 1.2 GB; a store hit after that), then `WARP_LIB=<worktree>/lib warp …`.
- Heavy jobs through /workspace/box-slot.sh. Worktrees with boxtree under /workspace/cow/trees/, never compacted.
  Kill processes by PID only.
- Routine builds and tests run at low effort with a step cap.

## Merging
- Workers land their own branches (Status above, common.md "Done"): no Ready column, no Integrator batch gate.
- Handovers of parked agents: /workspace/handover/<agent>.md.

## For the Supervisor
Boundary while the core extraction runs (user 2026-11-03): Grok coders work only in lib/, samples/, test/, the
playground (web/) and wiki/; never in core/ or extensions/ (merge conflicts with the extraction). Grok's machine
is its own, so load is no reason to limit them; conflicts are.
Grok Bot's Supervisor runs the Grok coders named in data/grok-cards.md; SendMessage can't reach it. Message it with
`grok-msg warp-supervisor "<text>"` (~/dev/bin/grok-msg, webhook; secrets in ~/.config/grok-bot/webhook, never in the
repo); it prints "sent to Grok Bot Supervisor" and wakes it through its fleet inbox. The webhook key has no expiry and
changes only when the user regenerates it: a 401 means exactly that, so ask the user to have Grok refresh the file
and write to data/grok-cards.md meanwhile (a 401 hid every message from 2026-11-02 to 11-03). Grok answers by typing
into the warp-supervisor tmux pane. Hand its coders simple library-only cards: `todo take <card> <coder>`, write the task into data/grok-cards.md,
ping with grok-msg; they land their branches themselves. At most 1 active + 1 queued card per Grok coder.
Unexplained direct pushes to main with the user's identity (g-* cards) may be Grok Bot: ask the user.

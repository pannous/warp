# The Grok Bot fleet

The roles in notes/roles.md describe the Claude sessions on the Mac. The Grok Bot fleet keeps those roles but does a
few things differently. Where this file and the role files disagree, Grok agents follow this file.

## Agents
- Supervisor: plans, turns the user's decisions into plans and cards, starts workers and gives each its role from
  notes/roles.md and notes/agents/<role>.md, adapted by this file.
- Integrator (also the Tester): the only agent that merges to main and runs the full suite.
- Planner: writes worker plans as board cards when the user asks for it.
- Workers: one plan, one branch, one worktree.
- No Interviewer: when in doubt, a worker asks the user directly.

## Plans and approvals (AGENTS.md)
- Every worker gets a plan, and everything in it is pre-approved. A worker's own idea is fine if it can be undone.
- No approval ping-pong: the per-change OKs (pass ban, core size limits) don't apply inside a plan.

## Status lives in git
- The status is the branch name, the commit messages and the board card, not chat.
- Ready: `todo ready <card> [branch]`: checks that the branch is pushed and merges cleanly into origin/main, records
  branch + tip on the card and moves it to Ready. The box `todo` has it too.
- Messages only for blocked or a decision; FYIs go out without waking the agent. The user hands out work between
  agents; agents keep messages to each other to a minimum.
- An agent stops only its own helper jobs, never with a stop that reaches other agents.

## The box
- The box can go away: commit and push all progress, notes and handovers right away (AGENTS.md).
- All agents share one Linux box. The shared binary is /workspace/cow/warp-rust-latest, refreshed by the Integrator
  with `/workspace/box-slot.sh /workspace/build-fresh-warp.sh` after pushed batches.
- Pure only: `warp rebuild pure` and `rebuild self` on the shared binary, with WARP_LIB=<worktree>/lib. No per-branch
  cargo builds unless a branch changes Rust.
- Every heavy run goes through /workspace/box-slot.sh (4 slots, memory guard). Box memory is the tight resource.
- Worktrees with boxtree under /workspace/cow/trees/, never compacted. Kill processes by PID only.
- Routine builds and tests run at low effort with a step cap.

## Merging
- The Integrator merges Ready cards in batches, once most of them are Ready, with one merge commit per branch.
- Every batch gate runs the gate set (test/gate) in both pure and self lanes. After a green batch the Integrator tags main
  `good/<date>`. A regression is bisected with scripts/bisect-lane.sh from the latest good tag.
- Handovers of parked agents: /workspace/handover/<agent>.md.

## For the Supervisor
Grok Bot's Supervisor runs the agents Warp Code, Warp Code 2, Warp Issues; SendMessage can't reach it. Message it with
`grok-msg warp-supervisor "<text>"` (~/dev/bin/grok-msg, webhook; secrets in ~/.config/grok-bot/webhook, never in the
repo); it replies in your tmux pane or via scratch/grok-cards.md. Its agents are our workers (user: "freely
send them cards"): first ask it whether they're busy and how many cards each can take, then `todo take <card>
grok-warp-code` / `grok-warp-code-2` / `grok-warp-issues`, write the task into scratch/grok-cards.md and ping with
grok-msg; they `todo ready`, the Integrator merges. Capacity: at most 1 active + 1 queued card per Grok coder; heavy
builds go to Warp Code 3's Linux box. Unexplained direct pushes to main with the user's identity (g-* cards) may be
Grok Bot: ask the user.

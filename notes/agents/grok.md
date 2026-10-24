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
- No approval ping-pong: the per-change OKs (pass ban, lib/compiler size limits) don't apply inside a plan.

## Status lives in git
- The status is the branch name, the commit messages and the board card, not chat.
- Ready: `todo add "merge: <branch> <sha>" Now -b "<summary, tests run>"`. The box `todo` has no `ready` command.
- Messages only for blocked or a decision; FYIs go out without waking the agent. The user hands out work between
  agents; agents keep messages to each other to a minimum.
- An agent stops only its own helper jobs, never with a stop that reaches other agents.

## The box
- The box can go away: commit and push all progress, notes and handovers right away (AGENTS.md).
- All agents share one Linux box. The shared binary is /workspace/cow/sol-warp-latest, refreshed by the Integrator
  with `/workspace/box-slot.sh /workspace/build-fresh-warp.sh` after pushed batches.
- Pure only: `warp rebuild pure` and `rebuild self` on the shared binary, with WARP_LIB=<worktree>/lib. No per-branch
  cargo builds unless a branch changes Rust.
- Every heavy run goes through /workspace/box-slot.sh (4 slots, memory guard). Box memory is the tight resource.
- Worktrees with boxtree under /workspace/cow/trees/, never compacted. Kill processes by PID only.
- Routine builds and tests run at low effort with a step cap.

## Merging
- The Integrator merges Ready cards in batches, once most of them are Ready, with one merge commit per branch.
- Every batch gate runs the probe set in both pure and self lanes. After a green batch the Integrator tags main
  `good/<date>`. A regression is bisected with scripts/bisect-lane.sh from the latest good tag.
- Handovers of parked agents: /workspace/handover/<agent>.md.

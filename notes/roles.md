# Agent roles in warp

Startup prompts and the spawn script for every role: notes/agents/ (README.md says how to start the team).
ListAgents shows the current sessions; each is named `warp-<role>`. The rules every session follows: AGENTS.md.

- Supervisor: owns the project's progress; turns requests into cards and tasks, spawns and assigns workers, watches
  load and the merge flow, decides technical and process questions itself (notes/agents/supervisor.md).
- Interviewer: the only session that asks the user; every other session sends it its question and never waits
  (notes/agents/interviewer.md).
- Integrator (also the Tester): applies and pushes the work handed to it, watches main, runs the full suite at rare
  intervals; tester and integrator are one role because the full suite only means something on the merged tree
  (notes/agents/integrator.md).
- Gatekeeper: sandboxed first judge of every outside ticket (notes/agents/gatekeeper.md).
- Fixer: small to-dos, one after another (notes/agents/fixer.md).
- Workers: one task, one branch, one worktree (notes/agents/worker.md).

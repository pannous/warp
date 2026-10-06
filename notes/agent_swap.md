# Automatic agent swap at 300k context

User 2026-10-06: 92% of the paused sessions ran above 150k context, 41% came from sessions active for 8+ hours; swap
agents automatically, quietly or with a notice to the team. Mechanism: `~/dev/bin/context-swap`, a global Stop hook
(`hooks.Stop` in ~/.claude/settings.json). Probe: `bash probes/context_swap_probe.sh` (real transcript and tmux, a spawner that
only records its arguments).

Flow, only for sessions inside tmux (started by claude-remote.sh); desktop sessions are untouched:
1. At every Stop the hook reads the context size of the last assistant turn from the transcript (input + cache read +
   cache creation tokens). Below SWAP_TOKENS (300000) it stays silent.
2. Above: it blocks the stop and asks the agent to write `~/.claude/handovers/<project>-<role>-<time>.md` (role = the tmux name
   minus the project prefix, e.g. `integrator`). Asked up to 3 times; then `SWAP STALLED` goes to the log and the session keeps running.
3. Handover exists: the old tmux session is renamed `<name>-retiring`, `claude-remote.sh <role> <dir> <prompt>` starts the replacement
   under the same tmux name. A failed spawn renames the old one back. The old one is killed once its background jobs
   are done: `context-swap --retire <tmux> <pane pid>` (detached) waits 20 s, then until no Bash-tool shell (a process
   under the pane whose command sources `~/.claude/shell-snapshots/`) is left, at most 30 min (`RETIRE TIMEOUT` in the
   log), then `RETIRED`. Killing it at once killed the Integrator's background ./test.sh (2026-10-06 17:20).
   The handover request also asks the agent to finish short background jobs and name the others in the handover
   (command, output file, what to do with the result); a job finishing after the swap only appends its result there.
4. The replacement's prompt makes its first action a SendMessage to the peers of the old session ("role X was replaced, address me by my
   ListAgents name"): that is the team notification. Everything is logged in `~/.claude/swap/swaps.log` (the noticeable trail).

Limits and traps:
- Peers address sessions by the generated name (warp-7a); a swap changes it. The notice covers it, and the tmux role stays stable.
- The handover is written by the agent itself, so it is only as good as the instruction text in the hook; check the first swaps by hand.
- Spawns ignore the CPU-load refusal of claude-remote.sh (FORCE_SPAWN=1): a swap replaces a session, it does not add one.
- The limit is read at every Stop from ~/.claude/swap/swap_tokens (beats the environment, so running sessions follow a change at once; currently 300000), else env SWAP_TOKENS, else 300000. SWAP_DRY_RUN=1 only logs. SWAP_RETIRE_GRACE / _POLL / _MAX_WAIT (seconds) shorten the retire wait in the probe.
- A session at the usage limit cannot write a handover: swapping needs headroom, so keep the limit well above 300k.

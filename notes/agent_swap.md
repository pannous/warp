# Automatic agent swap at 300k context

`~/dev/bin/context-swap`, a global Stop hook (`hooks.Stop` in ~/.claude/settings.json), swaps long-running sessions.
Probe: `bash probes/context_swap_probe.sh` (real transcript and tmux, a spawner that only records its arguments).

Flow, only for sessions inside tmux (started by claude-remote.sh); desktop sessions are untouched:
1. At every Stop the hook reads the context size of the last assistant turn from the transcript (input + cache read +
   cache creation tokens). Below the limit it stays silent.
2. Above: it blocks the stop and asks the agent to write `~/.claude/handovers/<project>-<role>-<time>.md` (role = the
   tmux name minus the project prefix, e.g. `integrator`), to finish short background jobs and to name the others in
   the handover (command, output file, what to do with the result). Asked up to 3 times; then `SWAP STALLED` goes to
   the log and the session keeps running.
3. Handover exists: the old tmux session is renamed `<name>-retiring`, `claude-remote.sh <role> <dir> <prompt>` starts
   the replacement under the same tmux name. A failed spawn renames the old one back. The old one is killed once its
   background jobs are done: `context-swap --retire <tmux> <pane pid>` (detached) waits 20 s, then until no Bash-tool
   shell (a process under the pane whose command sources `~/.claude/shell-snapshots/`) is left, at most 30 min
   (`RETIRE TIMEOUT` in the log), then `RETIRED`. Killing at once would kill e.g. a background ./test.sh.
4. The replacement's first action is a SendMessage to the peers of the old session: that is the team notification.
   Everything is logged in `~/.claude/swap/swaps.log`.

Limits and traps:
- The handover is written by the agent itself, so it is only as good as the instruction text in the hook.
- Spawns ignore the CPU-load refusal of claude-remote.sh (FORCE_SPAWN=1): a swap replaces a session, it does not add one.
- The limit is read at every Stop from ~/.claude/swap/swap_tokens (beats the environment, so running sessions follow a
  change at once), else env SWAP_TOKENS, else 300000. SWAP_DRY_RUN=1 only logs. SWAP_RETIRE_GRACE / _POLL / _MAX_WAIT
  (seconds) shorten the retire wait in the probe.
- A session at the usage limit cannot write a handover: swapping needs headroom, so keep the limit well above 300k.

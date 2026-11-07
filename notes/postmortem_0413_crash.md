# Postmortem: every warp agent session died at 04:13 (2026-11-07)

## Cause: one agent's `pkill -f "…" -n`, read by macOS pkill as a second pattern `-n`

At 04:13:34 warp-structure-item (transcript ff6f730b, then working as warp-sprint-media-samples) closed its card
fetch-users-async and ran:

```bash
todo done fetch-users-async a331690053 2>&1 | tail -1; agent-browser --session fetchprobe close >/dev/null 2>&1;
pkill -f "test_in_browser.py --serve" -n 2>/dev/null; pgrep -fl "…" | head -3
```

macOS (BSD) `pkill` stops reading options at the first pattern, so the trailing `-n` is **not** "newest only".
It is a second pattern. `-f` matches it against full command lines, and `-n` appears in every
`claude --name warp-…` line, in every Chrome flag line (`--no-…`) and in every `zsh -c` tool shell. So pkill sent
SIGTERM to every one of those processes. Checked this morning, read-only: `pgrep -f "zzz_no_such_thing" -n`
lists 63 processes, among them every running `claude --name warp-*`.

Why that one session survived: BSD pkill skips **its own ancestors** by default (`man pgrep`, `-a`). The ancestors
of the killing shell were the survivor's own Claude process and the tmux server (pid 11061, running since 11-04
19:53). So that Claude process and the tmux server lived. Every other Claude process in the same server got the
signal. When a Claude process exits, its pane closes and its tmux session goes with it. The tmux server never died,
and there was one socket only (`/private/tmp/tmux-501/default`).

Evidence:
- All dead transcripts end within 250 ms of each other, 04:13:40.77 to 04:13:41.03. Each one is a clean
  SIGTERM shutdown: its background tasks are reported `<status>killed</status>`, then the final `cost-state` and
  `bridge-session` records are written. That's a graceful exit, not a crash or OOM. The sessions were supervisor,
  interviewer, integrator, playground, sprint-web, -types, -units-time, -text-collections, -spec,
  -runtime-cleanup, app-webview/sugar, markup-handler-spacing and cursor-push-audit.
- The survivor's own tool call took 04:13:34 to 04:13:43 and returned only the `todo` line. Its `pgrep` printed
  nothing. Its background "Build playground and serve it locally" also died at 04:13:40.769 (exit 144).
- The unified log at 04:13:40.625 shows `loginwindow: appDeath for com.google.chrome.for.testing` (killed by the
  same pattern), then `MTLCompilerService … SIGKILLed during teardown … after host exited` at 04:13:41.48.
- Ruled out:
  - Jetsam or memory-pressure kills: the log has none in that window.
  - Sleep, logout or a panic.
  - A Claude Code update: 2.1.319 was installed 11-06 18:14.
  - `tmux kill-server` / `kill-session`: no script in scripts/ or ~/dev/bin, and no `todo` code, kills tmux
    sessions. `todo` only renames sessions and sends keys to them.
  - after-push.sh: it started at 04:13:31 for a331690053, 9 s earlier, and doesn't kill anything.

## What made it worse
- The rule already existed and was ignored. notes/agents/common.md says "Kill your own leftover processes by PID,
  never `pkill -f`", and the memory note never-pkill-by-pattern records the 10-02 self-kill.
- The GNU habit `pkill … -n` fails silently on macOS. The `2>/dev/null` hid everything, and the agent didn't
  notice that its own pgrep check printed nothing.
- The supervisor and integrator died too, so nothing noticed or restarted the fleet. Sessions were down from 04:13
  until the user restarted them about 06:26–06:32 (≈2 h 15 min). The survivor's report then failed: "the
  Supervisor session that assigned this has gone".
- A load of ~90–130 (11 sessions, each with builds, Chrome probes and samples-web sweeps) left many processes
  running. Each was a match for any broad pattern.

## Prevention
1. Make pattern kills impossible instead of relying on the rule. Add a PreToolUse Bash hook in the project's
   `.claude/settings.json` that refuses `pkill` and `killall` outright. Agents kill by PID, or stop a job through
   its task handle. A command line with `pkill -f` can never be safe on a Mac that is full of Claude sessions whose
   prompts mention everything.
2. Recover automatically. When a `warp-*` Claude process exits, a small watchdog run from cron (not by an agent)
   restarts it with `claude --resume <id>` (how the fleet was restarted at 06:32), or at least messages the user.
   Then a fleet-wide kill costs minutes, not two hours.

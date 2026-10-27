# Integrator handover (2026-10-27, before the user's reboot)

Role: notes/agents/integrator.md (self-landing: workers push, the Integrator watches main). Supervisor: warp-supervisor.

## State
- main is at the tip of origin/main; the user's checkout /Users/me/dev/angles/warp is pulled.
- Installed: ~/dev/bin/warp -> main-builds/769e12e71/warp (harness 769e12e71), warp.wasm of c0494668e6 beside it,
  warp-self.wasm missing (promote-self runs after each install). The seed: ~/dev/bin/warp-seed.wasm, newest release
  seed-2026-10-26-f8e6b127 on GitHub.
- after-push state in /Volumes/Worktrees/integrate2/data/integrate: watched (last installed) c0494668e6, handled
  (last started) a14f27c74b.

## Running at the reboot (all safe to lose)
- after-push.sh (cron run, pid 24429) for a14f27c74b, in `rebuild.sh both` for 31 min (slow: the Mac was loaded).
  Since c8aa082f14 its lock holds the pid, so the dead run's lock is taken over: the cron (every 2 min, `--if-moved`,
  log ~/Library/Logs/warp/cron/after-push.log) starts over by itself after the reboot. Nothing to do by hand.
  If the log still says "run … is going" with a dead pid: `rm -rf /Volumes/Worktrees/integrate2/data/integrate/lock`.

## After the reboot
- Check the cron log shows an install of origin/main's tip; `~/dev/bin/warp version` names it.
- If origin/main changed the harness since 769e12e71, after-push.sh builds and installs it itself (first real run of
  install_harness, dc139c22f7: watch its log line "harness of … installed").

## Pending
- Nothing in the merge queue (workers land their own branches). Worktree /Volumes/Worktrees/integrate (detached at
  b6b474e578) is only for landing branches of retired sessions (merge.sh/land.sh); can be recreated any time.

# Churn volumes: build output and junk off the Data volume

fseventsd logs every write on the Data volume (`.noindex` only stops Spotlight), and with many parallel agent sessions
the compiler builds, data/ output and temp files made it go wild. So churn lives on APFS volumes of their own (they
share the container's free space), each with `.fseventsd/no_log` at its root (needs sudo, then remount), Spotlight off,
Time Machine excluded:

| volume | disk | holds | wipe when |
|---|---|---|---|
| /Volumes/Junk | disk3s8 | the main checkout's data/ (a symlink to /Volumes/Junk/warp/data), throwaway lib/ copies (scripts/self-check.sh) | any time; worst case a running test fails and is rerun |
| /Volumes/Build | disk3s7 | the compiler store /Volumes/Build/compilers/<hash>/ (scripts/rebuild.sh; AGENTS.md "Testing") | only with no builds running; every compiler is built anew on demand |
| /Volumes/Worktrees | disk3s9 | new worktrees, `/Volumes/Worktrees/<name>` | never |

Setup, check and wipe: `~/dev/bin/churn-volume setup|check|wipe <volume>` (guide: ~/dev/bin/notes/apfs-churn-volumes.md).

- cowtree needs a registered donor worktree on the Worktrees volume, one per project named after it:
  /Volumes/Worktrees/warp, detached at origin/main, never removed (scripts/prune-worktrees.sh keeps it on origin/main).
- All worktrees live on the Worktrees volume, the infra trees too (integrate, integrate2).
- APFS clones (clonefile, `cp -c`, cowtree) work only inside one volume: a copy from Build to Worktrees, Junk or Data
  is a real copy.

Open: check that editors still get live file-change events on no_log volumes.

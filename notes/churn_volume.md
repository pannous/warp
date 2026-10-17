# Churn volumes: build output and junk off the Data volume

fseventsd logs every write on the Data volume (`.noindex` only stops Spotlight), and with many parallel agent sessions
the shared cargo target, scratch and temp files made it go wild. So churn lives on APFS volumes of their own (they
share the container's free space), each with `.fseventsd/no_log` at its root (needs sudo, then remount), Spotlight off,
Time Machine excluded:

| volume | disk | holds | wipe when |
|---|---|---|---|
| /Volumes/Junk | disk3s8 | scratch/ (the main checkout's scratch is a symlink to /Volumes/Junk/warp/scratch), test temp dirs (tests/queue.sh sets TMPDIR=/Volumes/Junk/tmp), cloud-test waiter files | any time; worst case a running test fails and is rerun |
| /Volumes/Build | disk3s7 | the shared cargo target (`target-dir` in ~/.cargo/config.toml; ~/.cargo/shared-target.noindex is a symlink to it) | only with an empty test queue and no builds (Integrator idle); costs one full rebuild |
| /Volumes/Worktrees | disk3s9 | new worktrees, `/Volumes/Worktrees/<name>` | never |

Wipe: `diskutil apfs deleteVolume <disk>`, then recreate and redo the setup (guide: ~/dev/bin/notes/apfs-churn-volumes.md).

- cowtree needs a registered donor worktree on the Worktrees volume, one per project named after it:
  /Volumes/Worktrees/warp, detached at origin/main, never removed (scripts/prune-worktrees.sh keeps it on origin/main).
- All worktrees live on the Worktrees volume, the infra trees too (integrate, integrate2,
  main-binary, nightly, trial); /Volumes/Worktrees/integrator is a plain folder with integrator_tools and the
  Integrator's logs, batch and held files.
- sccache's cache and data/ (kept logs and results) stay on Data.
- APFS clones (clonefile, `cp -c`, cowtree) work only inside one volume. Cargo's hard links stay inside the target dir.
  A copy from Build to Junk or Data (scripts/own-warp.sh → scratch/warp) is a real copy of ~225 MB.

Open: prune-cargo-target.sh could offer a full Build wipe when the target grows past its limit and the queue is
empty; check that editors still get live file-change events on no_log volumes.

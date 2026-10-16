# Churn volumes: build output and junk off the Data volume

Problem: after a few days of 20 parallel agent sessions fseventsd goes wild. `.noindex` only stops Spotlight;
fseventsd still logs every write on the Data volume (shared cargo target, scratch, temp files).

Fix: two APFS volumes (created 2026-10-16, both share the container's free space), each with
`.fseventsd/no_log` at its root (fseventsd keeps no log; needs sudo, then remount), Spotlight off, Time Machine excluded.
They differ in when they may be wiped (`diskutil apfs deleteVolume <disk>`, then recreate and redo the setup):

| volume | disk | holds | wipe when |
|---|---|---|---|
| /Volumes/Junk | disk3s8 | scratch/, test and probe temp dirs, $TMPDIR warp*, cloud-test waiter files | any time; worst case a running test fails and is rerun |
| /Volumes/Build | disk3s7 | shared cargo target, sccache cache | only with an empty test queue and no builds (Integrator idle); costs one full rebuild |

Old worktrees (/Users/me/dev/angles/warp.worktrees.noindex) stay on the Data volume until retired; NEW worktrees go
straight to /Volumes/Worktrees/<name> (disk3s9, user 2026-10-16, card new-worktrees): no_log, Spotlight off, Time Machine
excluded for the whole volume (user: work is pushed per step), no .noindex subfolder. Never wiped. cowtree needs a
registered donor worktree on that volume, one per project named after it: /Volumes/Worktrees/warp, detached at origin/main
(the volume may hold other projects later).
data/ (kept logs and results) stays in the repo.

## Status
- [x] volumes created, Spotlight off
- [x] user (sudo), remounted 2026-10-16: no_log + Time Machine exclusion on both
- [x] 2026-10-16 22:03: target-dir in ~/.cargo/config.toml → /Volumes/Build/target (no copy: the old
  ~/.cargo/shared-target.noindex was 204 GB; sccache makes the cold rebuild cheap); prune-cargo-target.sh reads the
  target dir from the config
- [ ] delete the old ~/.cargo/shared-target.noindex once `lsof +D` shows nothing in it, then symlink the old path to
  /Volumes/Build/target (probes and clean-git-lost-found.sh name it)
- [ ] scratch/ → /Volumes/Junk (later; sccache stays on Data, 1 GB, little churn)

## Copy-on-write
APFS clones (clonefile, `cp -c`, cowtree) work only inside one volume, not across volumes of the same container.
So worktrees stay on Data next to the main checkout (cowtree clones it). Cargo's hard links (deps → uplifted binary,
the tests' own_copy into target tmp/) stay inside the target dir, so they keep working on Build. A copy from Build to
Data or Junk (scripts/own-warp.sh → scratch/warp) is a real copy of ~225 MB.
- [ ] scripts/prune-cargo-target.sh: offer a full Build wipe when the target grows past its limit and the queue is empty
- [ ] check that editors still get live file-change events there (no_log should only stop the on-disk log)

General guide (setup commands, copy-on-write rules, reuse for other projects): ~/dev/bin/notes/apfs-churn-volumes.md

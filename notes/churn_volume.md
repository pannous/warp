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

Worktrees (warp.worktrees.noindex) stay on the Data volume: they hold uncommitted work, so they must never be
wiped with a volume, and Time Machine should keep backing them up. Their churn is small next to the target dir.
data/ (kept logs and results) stays in the repo.

## Status
- [x] volumes created, Spotlight off
- [x] user (sudo), remounted 2026-10-16: no_log + Time Machine exclusion on both
- [ ] move when the fleet is quiet: target-dir in ~/.cargo/config.toml → /Volumes/Build/target (no copy: the old
  ~/.cargo/shared-target.noindex was 223 GB with 93 GB free; sccache makes the cold rebuild cheap), then delete the old
  dir once `lsof +D` shows no cargo/rustc in it; SCCACHE_DIR → /Volumes/Build/sccache; scratch/ → /Volumes/Junk/warp-scratch
- [ ] scripts/prune-cargo-target.sh: offer a full Build wipe when the target grows past its limit and the queue is empty
- [ ] check that editors still get live file-change events there (no_log should only stop the on-disk log)

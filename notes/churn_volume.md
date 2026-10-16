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
- [ ] user (sudo), then remount both:
  `for v in Build Junk; do sudo touch /Volumes/$v/.fseventsd/no_log; sudo tmutil addexclusion -v /Volumes/$v; done`
- [ ] move when the fleet is quiet, symlink the old paths: target-dir in ~/.cargo/config.toml → /Volumes/Build/target,
  SCCACHE_DIR → /Volumes/Build/sccache, scratch/ → /Volumes/Junk/warp-scratch
- [ ] scripts/prune-cargo-target.sh: offer a full Build wipe when the target grows past its limit and the queue is empty
- [ ] check that editors still get live file-change events there (no_log should only stop the on-disk log)

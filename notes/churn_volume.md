# Churn volume: build output and worktrees off the Data volume

Problem: after a few days of 20 parallel agent sessions fseventsd goes wild. `.noindex` only stops Spotlight;
fseventsd still logs every write on the Data volume (shared cargo target, worktrees, scratch).

Fix: a separate APFS volume `/Volumes/Churn` (disk3s7, created 2026-10-16, shares the container's free space)
- `.fseventsd/no_log` at its root: fseventsd keeps no log for the volume (needs sudo, then remount)
- Spotlight off (`mdutil -i off`), Time Machine excluded (`sudo tmutil addexclusion -v`)
- disposable by design: everything on it can be rebuilt; to wipe the junk, delete and recreate the volume
  (`diskutil apfs deleteVolume disk3s7`, then the setup above), no rm -rf over millions of files

## Status
- [x] volume created, Spotlight off
- [ ] user: `sudo touch /Volumes/Churn/.fseventsd/no_log && sudo tmutil addexclusion -v /Volumes/Churn`, remount
- [ ] move when the fleet is quiet (no builds running), symlink the old paths:
  - shared cargo target `~/.cargo/shared-target.noindex` (or set `target-dir` in ~/.cargo/config.toml)
  - worktrees `/Users/me/dev/angles/warp.worktrees.noindex`
  - `scratch/`, test queue temp files, sccache dir
- [ ] check that editors still get live file-change events there (no_log should only stop the on-disk log)

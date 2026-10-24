# Warp Scripts handover (2026-10-23 14:45 CEST)

Status: parked. No running jobs. Reports went to Supervisor 2 (a0511d7c-163b-4eb4-a8dc-5a4efb46cf50).

## Branch `script-ergonomics` (pushed, NOT yet in main as of 14:45)
origin/main..origin/script-ergonomics, tip 724d3fa96:
- f240fc5af scripts/lib/quiet.sh shared helper (summary line, failing lines, named signal deaths)
- 129c4da93 tests/queue.sh quiet default + -h (full output: VERBOSE=1, --nocapture/--show-output, stdout to file); probes/fd_limit.sh sets VERBOSE=1
- e7d753ed8 tests/nightly.sh exit = test status (was 0 on red)
- ab8ed13a0 cloud-test.sh -h
- 7c5008ece self-hosted-suite.sh --local fails on killed/unbuilt run (line ~141)
- bb493bada lib_bench.sh pipefail, -h, killed batch -> exit 1
- 712ad79ad cloud-test.sh unknown leading option -> usage, exit 2
- 7696932e5 test.sh quiet default; full log always in data/logs/test-<time>-<pid>.log; exit status unchanged
- 724d3fa96 merge origin/main
Ignore hashes c5ae55265 4c6834c94 150b6ef9a 7290f9eb4 c36e560b9 (local rebased copies, never pushed).

## Board
Card hC_asg "script-ergonomics: token-lean script output + honest exit codes" (Now). Box `todo` has no take/ready:
on the Mac run `todo take ... grok-warp-scripts` and `todo ready ... script-ergonomics --express` (may need `todo key` rename first).

## Box scripts edited in place (atomic mv)
- /workspace/box-slot.sh: quiet default, log /tmp/warp-logs/slot-<ticket>-<cmd>.log, real rc, OOM via dmesg + 2s process-tree sample -> "killed: out of memory" rc 137, forwards TERM/INT/HUP, -h. --status, queue, memory guard unchanged. Verified on live queue (pass rc0, fail rc4). Backups: /workspace/script-backups/box-slot.sh.20261023-142843, /workspace/box-slot.sh.bak-20261023-142843-grokscripts
- /workspace/build-fresh-warp.sh: quiet, real build errors shown, real rc. Backup /workspace/script-backups/build-fresh-warp.sh.*
- /workspace/install-fresh-warp.sh: smoke failure now exits nonzero. Backup /workspace/script-backups/install-fresh-warp.sh.20261023-142843

## Caveats / open
- Quiet mode turns off when stdout is a regular file; a gate reading a pipe should set VERBOSE=1.
- test.sh still exits 0 when tests fail (pre-existing, kept on purpose); summary says FAILED TESTS. Its `sed -i ''` is Mac-only.
- box-slot OOM detection misses victims living <2s; /tmp/warp-logs is never pruned.
- A grandchild outliving its job still holds the slot lock (pre-existing).
- BOX_SLOT_* env vars are test overrides; only set them in a subshell.
- Not done: install-fresh-warp.sh bypasses the queue via /tmp/warp-box-heavy.lock (maybe obsolete; ask owner, Supervisor said leave it). self-check.sh has no -h (first arg is a path).
- After merge: `boxtree rm script-ergonomics` (worktree /workspace/cow/trees/script-ergonomics).
- Notes: /workspace/warp-scripts-audit.md (ranked list + per-script table), /workspace/warp-scripts-progress.md. todo.md donor diff saved at /workspace/script-backups/todo.md.donor-diff.*

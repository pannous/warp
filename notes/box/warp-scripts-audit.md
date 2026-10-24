# warp scripts audit (grok-warp-scripts, 2026-10-23)

## Top: remaining ergonomics fixes, ranked by estimated token savings
1. **test.sh (full suite, Integrator)**: streams every test line (~3-5k lines/run) through `tee`. Quiet mode via
   scripts/lib/quiet.sh (summary file already exists) would save the most per run, but the Integrator's gate on the Mac
   may parse its stdout: needs warp-integrator's OK. tests/queue.sh keeps the full ./test.sh streaming for now.
2. **build-fresh-warp.sh (box)**: already short (~10 lines), but `cargo build | tail -3` hides the real error on a
   failed build; run it through box-slot (now logs everything) and print `tail -3` only on success. Small saving.
3. **scripts/lib_bench.sh**: no --help, no pipefail (parse_files pipe loses timeout/warp status, :46); prints 2 lines
   + report file: fine on volume.
4. **scripts/self-check.sh / slot.sh / selfcheck-box.sh**: already one verdict line; no change needed. self-check.sh
   has no -h (its first arg is a worktree path; `-h` would be taken as a path).
5. **scripts/cloud-test.sh**: lane paused; result already summarized in result.md. Unknown leading `--opt` is taken
   as the branch name (:880 `*) break`), no error. Low priority while paused.
6. **install-fresh-warp.sh (box)**: superseded by build-fresh-warp.sh and still takes the bare
   /tmp/warp-box-heavy.lock (bypasses the FIFO queue); candidate for deletion (ask the owner).

## Done in this pass
- /workspace/box-slot.sh: quiet by default (log /tmp/warp-logs/slot-<ticket>-<cmd>.log), one summary line, <=30 error
  lines on failure, VERBOSE=1 / stdout-to-file keep full output, signal deaths named, OOM of any process in the job's
  tree (dmesg + process-tree sampler) => rc 137 even if the job exited 0, TERM/INT/HUP forwarded to the job, --help.
  `--status`, queue and memory guard unchanged.
- /workspace/install-fresh-warp.sh: exited 0 after a failed smoke rebuild (last command `git status | head`) -> exit $rc.
- branch script-ergonomics: scripts/lib/quiet.sh; tests/queue.sh quiet default + --help; tests/nightly.sh exit status;
  scripts/self-hosted-suite.sh --local exit status + --help; scripts/cloud-test.sh --help.

## Per-script audit (default output / --help / log / verbose / exit-code bugs)
| script | default output | --help | log file | verbose | exit-code issues |
|---|---|---|---|---|---|
| /workspace/box-slot.sh (before) | full job output | no (ran `--help` as a command) | no | n/a | rc passed through (`"$@"; rc=$?` :221), but an OOM-killed sub-step of a job that exits 0 was invisible; SIGTERM to the wrapper orphaned the job |
| tests/queue.sh (before) | full cargo output, prebuild compile lines too (100s-1000s of lines) | no (went to cargo test --help) | no | n/a | exec keeps status (ok); watch_run kill -9 => 137 (ok) |
| tests/runner.sh | none (exec "$@") | n/a | n/a | n/a | none |
| tests/nightly.sh | 2 lines, log in data/nightly | no | yes | n/a | exited with `todo add`'s status: a red nightly whose card was filed exited 0 (end of file) |
| test.sh | full suite (thousands of lines) via tee :73 | no | TEMP_FILE + summary | no | no pipefail on `run_test_executables | tee` :73, but crashes are counted as INCOMPLETE from the log, so status is derived from counts (ok) |
| scripts/self-check.sh | 1 verdict line | no | no | no | ok (`exit $passed`) |
| scripts/slot.sh | the command's | no | no | no | ok |
| scripts/cloud-test.sh | progress + result summary | no -> added | data/cloud/<run>/ | no | pipefail set; unknown leading option becomes the branch (:880) |
| scripts/self-hosted-suite.sh | ranked report | `--*` printed usage with exit 2 | data/self-hosted | no | --local ignored tests/queue.sh's status (:141): killed/unbuilt run exited 0 |
| scripts/lib_bench.sh | 2 lines + report | no | report file | no | no pipefail; parse_files pipeline (:46) |
| scripts/browser-dry-run.sh | moderate | own flags, no -h | ? | no | `gh auth token || true` (:92) intentional |
| scripts/own-warp.sh, prune-worktrees.sh, version-tweak.sh | short | no | no | no | none found |
| /workspace/build-fresh-warp.sh | ~10 lines | no | no | no | ok (pipefail on cargo|tail) |
| /workspace/install-fresh-warp.sh | ~10 lines | no | no | no | exited 0 after failed smoke rebuild -> fixed |
| /workspace/selfcheck-branch.sh / selfcheck-box.sh | ~10 lines | usage only | no | no | ok |

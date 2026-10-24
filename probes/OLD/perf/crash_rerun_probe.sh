#!/bin/bash
# test.sh's rerun of a crashed test binary, on probes/perf/crash_probe.rs: the rerun names the failed test with its
# panic and the tests running when the binary crashed again. Run from the repo root.
WORK=scratch/perf/crash_probe
mkdir -p $WORK
rustc --edition 2021 --test probes/perf/crash_probe.rs -o $WORK/crash_probe || exit 1
TEST_EXECUTABLES=$WORK/executables
TEMP_FILE=$WORK/log
printf '%s\t%s\n' "$PWD/probes/perf/crash_probe.rs" "$PWD/$WORK/crash_probe" > $TEST_EXECUTABLES
{ echo "     Running probes/perf/crash_probe.rs ($PWD/$WORK/crash_probe)"; $WORK/crash_probe --test-threads=4 2>&1; } > $TEMP_FILE
RUNNER_PROGRAM=env
TEST_THREADS=4
eval "$(sed -n '/^rerun_incomplete_executables() {$/,/^}$/p' test.sh)"
rerun_incomplete_executables | tee $WORK/report
grep -q 'fails_with_a_panic: .*arithmetic is broken' $WORK/report &&
	grep -q 'CRASHED again, while running: .*aborts_the_process' $WORK/report && echo "PROBE OK" || { echo "PROBE FAILED"; exit 1; }

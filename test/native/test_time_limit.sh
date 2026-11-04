#!/usr/bin/env bash
# A `warp test` case that spins uses its CPU budget and is a TIME with that reason, neither passed nor failed: the run
# still exits 0 and counts it apart. Run it with bash test/native/test_time_limit.sh (it spins half a minute); the warp
# under test is WARP (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
cases=$(cd "$(dirname "$0")/../.." && pwd)/data/time-limit
mkdir -p "$cases"
printf "'i = 0; while yes { i += 1 }; i'\t1\n'1 + 1'\t2\n" >"$cases/test_spin.pairs.warp"
said=$("$warp" test "$cases" 2>&1); status=$?
status_of_test=0
expect() {
	case $said in
		*"$1"*) echo "✓ $1" ;;
		*) echo "✗ expected …$1…, got: $said"; status_of_test=1 ;;
	esac
}
expect "TIME $cases/test_spin.pairs.warp:1"
expect "used its 30 s of CPU"
expect "✓ 1 tests passed in 1 files, 1 hit a time limit"
[ $status -eq 0 ] && echo "✓ exit 0" || { echo "✗ exit $status"; status_of_test=1; }
exit $status_of_test

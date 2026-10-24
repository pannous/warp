#!/bin/bash
# The pure-hello cloud preset selects only the ignored end-to-end test and gives it the optimized test profile.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main \"\$@\"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"

launch() { printf '%s\n' "$@"; }
collect() { :; }
expected=(example-branch 0123456789abcdef0123456789abcdef01234567 --config profile.test.package.warp-lang.opt-level=2 -- parser::test_warp_pure::warp_pure_runs_hello_world --ignored --exact)
actual=$(main --no-wait --pure-hello "${expected[0]}" "${expected[1]}")
[ "$actual" = "$(printf '%s\n' "${expected[@]}")" ] && echo "pure hello cloud preset probe: ok" || {
	echo "pure hello cloud preset probe: FAIL"
	printf 'expected:\n%s\nactual:\n%s\n' "$(printf '%s\n' "${expected[@]}")" "$actual"
	exit 1
}
[[ --config =~ ^($CARGO_FLAGS_WITH_VALUE)$ ]] || { echo "pure hello cloud preset probe: FAIL, --config is parsed as a filter"; exit 1; }
rejected=$(main --no-wait --pure-hello "${expected[0]}" "${expected[1]}" another-test 2>&1 || true)
[[ "$rejected" == *"does not take filters"* ]] || { echo "pure hello cloud preset probe: FAIL, extra filters accepted"; exit 1; }

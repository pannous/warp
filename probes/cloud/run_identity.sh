#!/bin/bash
# Cloud runs of one tip with different test settings need distinct result ids; an identical invocation keeps one id.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main "\$@"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)

branch=self-hosted/example
tip=0123456789abcdef0123456789abcdef01234567
plain=$(run_name "$branch" "$tip" --all-features -- parser::)
same=$(run_name "$branch" "$tip" --all-features -- parser::)
pure=$(run_name "$branch" "$tip" --env WARP_SELF_HOSTED=pure --all-features -- parser::)
other_filter=$(run_name "$branch" "$tip" --all-features -- emitter::)

fail() { echo "run identity probe: FAIL: $*"; exit 1; }
[ "$plain" = "$same" ] || fail "an identical invocation changed id"
[ "$plain" != "$pure" ] || fail "an environment setting did not change id"
[ "$plain" != "$other_filter" ] || fail "a filter did not change id"
[[ "$plain" =~ ^self-hosted-example-${tip:0:8}-[0-9a-f]+$ ]] || fail "unsafe or unreadable id: $plain"
echo "run identity probe: ok"

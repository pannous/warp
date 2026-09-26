#!/bin/bash
# Evaluates every footgun case from footguns.md with the real warp binary.
# Usage: probes/footguns/footguns.sh [> probes/footguns/results.txt]
# Cases are separated by lines containing only '---' so multi-line programs work.

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WARP="$REPO_ROOT/target/debug/warp"
CASES="$(dirname "$0")/cases.warp"
TIMEOUT_SECONDS=60

cargo build --offline --quiet --manifest-path "$REPO_ROOT/Cargo.toml" || exit 1

evaluate() {
	local code="$1"
	local result
	result=$(timeout "$TIMEOUT_SECONDS" "$WARP" eval "$code" 2>&1 | grep -av '^note:\|^$' | perl -pe 's/\0/\\0/g' | tail -1)
	printf '%-45s => %s\n' "${code//$'\n'/⏎}" "$result"
}

code=""
while IFS= read -r line || [ -n "$line" ]; do
	if [ "$line" = "---" ]; then
		[ -n "$code" ] && evaluate "$code"
		code=""
	elif [ -z "$code" ]; then
		code="$line"
	else
		code="$code"$'\n'"$line"
	fi
done < "$CASES"
[ -n "$code" ] && evaluate "$code"

#!/usr/bin/env bash
# Inline code exits 0 whatever its value, as warp-rust's inline code (src/cli/display.rs show_and_fail_on_error): only
# an Error or the program's exit(code) give another status; a file's Int stays its status (card warp-host-inline-status).
# Run it through the queue: tests/queue.sh tests/host/test_host_inline_status.sh
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED
[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
work=$checkout/scratch/host-inline-status-test
rm -rf "$work" && mkdir -p "$work" && cd "$work" || exit 1
printf '%s\n' '6*7' >answer.warp

run=0 failed=0
# expect <name> <exit status> <text the output holds> -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=$3 said got
	shift 4
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	if [ "$got" = "$status" ] && [[ $said == *"$wanted"* ]]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status), wanted [$wanted]"
		echo "$said" | sed 's/^/    /'
	fi
}

expect "inline code: its Int printed, status 0" 0 "42" -- "$host" run '6*7'
expect "no command: inline code, status 0" 0 "42" -- "$host" '6*7'
expect "inline code: exit(3) its status" 3 "" -- "$host" 'exit(3)'
expect "inline code: an error, status 1" 1 "" -- "$host" 'undefined_word_here()'
expect "a file: its Int still its status" 42 "42" -- "$host" run answer.warp

echo "$((run - failed))/$run passed"
[ "$failed" = 0 ]

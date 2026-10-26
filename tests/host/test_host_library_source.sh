#!/usr/bin/env bash
# The embedded std modules (prelude, math, list, regex) come from the run's lib/, the text inlined at build time only a
# fallback (card compiler-library-source, notes/compiler_rebuilds.md): an alias added to a temporary lib's prelude.warp
# works with a warp-self.wasm built before it, named by WARP_LIB. Run it through the queue:
# tests/queue.sh tests/host/test_host_library_source.sh
# The host and the compiler as in test_host.sh: WARP_HOST_BINARY (default target/host/warp-host, else target/release/,
# else target/debug/), warp-self.wasm from beside WARP_LAUNCHER_RUST (default target/debug/warp).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/host-library-source-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler=$(dirname "$rust")/warp-self.wasm
[ -f "$compiler" ] || { echo "✗ no warp-self.wasm beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/work" || exit 1
cp -R "$checkout/lib" "$stage/lib" || exit 1
# an alias no lib/ has, in a module the compiler used to read only from its build-time copy
printf '\n%s\n' 'alias howmany = count' >>"$stage/lib/prelude.warp"
cp "$host" "$stage/bin/warp-host" && cp "$compiler" "$stage/bin/" || exit 1
printf '%s\n' 'xs = [1, 2, 3]' 'print howmany xs' >"$stage/work/howmany.warp"
printf '%s\n' 'xs = [1, 2, 3]' 'print count xs' >"$stage/work/count.warp"
cd "$stage/work" || exit 1

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

expect "the plain word, as a baseline" 0 "3" -- env WARP_LIB="$stage/lib" "$stage/bin/warp-host" run count.warp
expect "WARP_LIB's prelude.warp: its new alias" 0 "3" -- env WARP_LIB="$stage/lib" "$stage/bin/warp-host" run howmany.warp

echo "$((run - failed))/$run passed"
[ "$failed" = 0 ]

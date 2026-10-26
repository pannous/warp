#!/usr/bin/env bash
# The values warp-host keeps for a compiler (compiler.warp's lowered runtime words, cache.rs Keeper) are keyed by what
# the compile reads from lib/ (core, the aliases, the prelude and the std modules the compiler uses), not by
# all of lib/ (card warp-host-cache-key): after an edit of another std module a run still prints its value and keeps
# nothing new; after an edit in core it keeps the values again.
# The host under test is WARP_HOST_BINARY (default the checkout's target/host/warp-host), with the compiler pair from
# WARP_HOST_COMPILERS (default the checkout's data/), and a copy of the checkout's lib/, staged in
# scratch/host-cache-key-test/. Run it in a heavy slot: /workspace/box-slot.sh tests/host/test_host_cache_key.sh
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
compilers=${WARP_HOST_COMPILERS:-$checkout/data}
stage=$checkout/scratch/host-cache-key-test
[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
for compiler in warp.wasm warp-self.wasm; do
	[ -f "$compilers/$compiler" ] || { echo "✗ no $compiler in $compilers: scripts/rebuild.sh copies the pair"; exit 1; }
done
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/cache" || exit 1
cp "$host" "$stage/bin/warp-host" && cp "$compilers"/warp.wasm "$compilers"/warp-self.wasm "$stage/bin/" && cp -RL "$checkout/lib" "$stage/lib" || exit 1
export WARP_HOST_CACHE=$stage/cache WARP_LIB=$stage/lib WARP_NO_WINDOW=1 WARP_HINTS=0
cd "$stage" || exit 1
printf '%s\n' '6 * 7' >program.warp

run=0 failed=0
check() { # check <name> <wanted> <got>
	run=$((run + 1))
	if [ "$2" = "$3" ]; then echo "✓ $1"; else failed=$((failed + 1)); echo "✗ $1: [$3], not [$2]"; fi
}
kept() { find "$stage/cache" -name '*.kept' | wc -l | tr -d ' '; }
ran() { timeout 600 "$stage/bin/warp-host" program.warp 2>/dev/null | tail -1; }

check "a run prints its value" 42 "$(ran)"
first=$(kept)
check "the run kept the compiler's values" yes "$([ "$first" -gt 0 ] && echo yes || echo no)"
echo '# an edit outside what the compiler reads' >>lib/sound.warp
check "after a std module's edit the run prints its value" 42 "$(ran)"
check "after a std module's edit nothing new is kept" "$first" "$(kept)"
echo '# an edit of what the compiler reads' >>core/runtime/README.md
check "after an edit in core the run prints its value" 42 "$(ran)"
check "after an edit in core the values are kept again" yes "$([ "$(kept)" -gt "$first" ] && echo yes || echo no)"

echo "$((run - failed)) of $run passed"
[ "$failed" = 0 ]

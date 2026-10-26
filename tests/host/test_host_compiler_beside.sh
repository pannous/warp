#!/usr/bin/env bash
# warp-host finds its compilers beside itself (card warp-host-compiler-beside): warp-self.wasm in warp-host's own
# folder, a symlink to warp-host or to its folder followed, from any working directory, before <lib>/../data. And the
# build put the pair beside the binary it made. Run it through the queue: tests/queue.sh tests/host/test_host_compiler_beside.sh
# The host as in test_host.sh: WARP_HOST_BINARY (default target/host/warp-host); the compilers from beside it, else
# WARP_HOST_COMPILERS.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
compilers=${WARP_HOST_COMPILERS:-$(dirname "$host")}
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$compilers/warp-self.wasm" ] || { echo "✗ no warp-self.wasm in $compilers"; exit 1; }
# outside the checkout: no lib/ above the working folder, so warp-host takes the lib/ beside it
stage=$(cd "$(mktemp -d)" && pwd -P) || exit 1  # symlinks resolved, as warp-host names its folder
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/install" "$stage/elsewhere" "$stage/links" || exit 1
# an installed layout: warp-host, its compilers and lib/ in one folder, nothing of the checkout in reach
cp "$host" "$compilers/warp-self.wasm" "$stage/install/" && cp -RL "$checkout/lib" "$stage/install/lib" || exit 1
ln -s "$stage/install" "$stage/links/folder" && ln -s "$stage/install/warp-host" "$stage/links/warp-host" || exit 1

run=0 failed=0
# expect <name> <exit status> <text the output holds>… -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() said got text problems=""
	shift 2
	while [ "$1" != -- ]; do wanted+=("$1"); shift; done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	[ "$got" = "$status" ] || problems+=" exit $got, not $status"
	if [ -z "$problems" ]; then echo "✓ $name"; else failed=$((failed + 1)); echo "✗ $name:$problems"; echo "$said" | head -5 | sed 's/^/    /'; fi
}
in_elsewhere() { (cd "$stage/elsewhere" && "$@"); }

expect "the build put warp-self.wasm beside warp-host" 0 -- test -f "$(dirname "$host")/warp-self.wasm"
expect "the build put warp.wasm beside warp-host" 0 -- test -f "$(dirname "$host")/warp.wasm"
expect "from another folder: the compiler beside warp-host" 0 "49" -- in_elsewhere "$stage/install/warp-host" '7*7'
expect "through a symlinked folder" 0 "49" -- in_elsewhere "$stage/links/folder/warp-host" '7*7'
expect "through a symlink to warp-host" 0 "49" -- in_elsewhere "$stage/links/warp-host" '7*7'
# a broken compiler where a checkout keeps its own (<lib>/../data): the one beside warp-host comes first
mkdir -p "$stage/install/data" && printf 'not a module' >"$stage/install/data/warp-self.wasm"
expect "beside warp-host before <lib>/../data" 0 "49" "warp-host: $stage/install/warp-self.wasm with" \
	-- in_elsewhere env WARP_HOST_DEBUG=1 "$stage/links/warp-host" '7*7'
rm "$stage/install/warp-self.wasm"
expect "none beside: <lib>/../data's is the one tried" 1 "warp-host: $stage/install/lib/../data/warp-self.wasm with" \
	-- in_elsewhere env WARP_HOST_DEBUG=1 "$stage/install/warp-host" '7*7'

echo "$((run - failed))/$run passed"
[ "$failed" = 0 ]

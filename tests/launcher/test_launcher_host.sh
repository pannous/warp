#!/usr/bin/env bash
# Where the launcher packaging/bin/warp sends each command when warp-host is beside it (card launcher-host-dispatch):
# run, pure, self, compile, inline code and a .wasm module to warp-host; rebuild to the checkout's scripts/rebuild.sh;
# test to WARP_TEST_RUNNER, else warp-rust; every other command to warp-rust. Results only: which program ran it and
# with what, as stand-ins print it. With WARP_HOST_BINARY and a compiler pair (WARP_HOST_COMPILERS, default the
# checkout's data/) it also runs a program through the real warp-host. Light: stand-ins only, unless WARP_HOST_BINARY.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
stage=$checkout/scratch/launcher-host-test
export WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_RUST WARP_HOST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED WARP_LIB WARP_TEST_RUNNER

rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/tree/scripts" "$stage/tree/lib/compiler" || exit 1
cp "$checkout/packaging/bin/warp" "$stage/bin/" || exit 1
stand_in() { # stand_in <file> <name>: a program printing "<name> <its arguments>"
	printf '#!/usr/bin/env bash\necho "%s $*"\n' "$2" >"$1" && chmod +x "$1"
}
stand_in "$stage/bin/warp-host" host
stand_in "$stage/bin/warp-rust" rust
stand_in "$stage/tree/scripts/rebuild.sh" rebuild.sh
stand_in "$stage/runner" runner
git -C "$stage/tree" init -q || exit 1 # the checkout rebuild finds: the stand-in's, never this one
printf '%s\n' '6 * 7' >"$stage/tree/six.warp"
printf '\0asm\1\0\0\0' >"$stage/tree/six.wasm"
warp=$stage/bin/warp
cd "$stage/tree" || exit 1

run=0 failed=0
# is <name> <printed> -- <command…>: the command's last line of output is <printed> (first: its first line)
is() {
	local name=$1 wanted=$2 said line=${line:-tail}
	shift 3
	said=$("$@" 2>/dev/null | $line -1)
	run=$((run + 1))
	if [ "$said" = "$wanted" ]; then echo "✓ $name"; else failed=$((failed + 1)); echo "✗ $name: printed [$said], not [$wanted]"; fi
}

is "a program file: warp-host" "host run six.warp" -- "$warp" six.warp
is "run: warp-host" "host run six.warp" -- "$warp" run six.warp
is "pure: warp-host with compiler.wasm" "host --compiler full run six.warp" -- "$warp" pure six.warp
is "self: warp-host with compiler-self.wasm" "host run six.warp" -- "$warp" self six.warp
is "--fuel goes to warp-host" "host run --fuel 99 six.warp" -- "$warp" --fuel 99 six.warp
is "inline code: warp-host" "host run 6*7" -- "$warp" '6*7'
line=head is "compile --wasm: warp-host" "host compile six.warp six.wasm" -- "$warp" compile --wasm six.warp
is "a .wasm module: warp-host" "host six.wasm" -- "$warp" six.wasm
is "rebuild: scripts/rebuild.sh of the checkout" "rebuild.sh self $stage/tree" -- "$warp" rebuild self
is "rebuild with words rebuild.sh lacks: warp-rust" "rust rebuild --verbose" -- "$warp" rebuild --verbose
is "test: warp-rust for now" "rust test six.warp" -- "$warp" test six.warp
is "test: WARP_TEST_RUNNER" "runner six.warp" -- env WARP_TEST_RUNNER="$stage/runner" "$warp" test six.warp
is "version: warp-rust" "rust version" -- "$warp" version
is "a plain word: warp-rust" "rust pi" -- "$warp" pi
is "WARP_RUST names warp-rust" "runner version" -- env WARP_RUST="$stage/runner" "$warp" version
is "WARP_HOST=none: pure is warp-rust's" "rust pure six.warp" -- env WARP_HOST=none "$warp" pure six.warp

if [ -n "${WARP_HOST_BINARY-}" ]; then
	compilers=${WARP_HOST_COMPILERS:-$checkout/data}
	mkdir -p "$stage/real" && cp "$WARP_HOST_BINARY" "$stage/real/warp-host" && cp "$checkout/packaging/bin/warp" "$stage/real/" &&
		cp "$compilers"/compiler.wasm "$compilers"/compiler-self.wasm "$stage/real/" || exit 1
	export WARP_LIB=$checkout/lib WARP_RUST=$stage/bin/warp-rust WARP_HOST_CACHE=$checkout/scratch/host-cache
	is "pure through the real warp-host" 42 -- "$stage/real/warp" pure six.warp
	is "self through the real warp-host" 42 -- "$stage/real/warp" self six.warp
	is "inline code through the real warp-host" 42 -- "$stage/real/warp" '6*7'
fi

echo "$((run - failed)) of $run passed"
[ "$failed" = 0 ]

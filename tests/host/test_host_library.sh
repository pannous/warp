#!/usr/bin/env bash
# The compiler run reads the std modules from the lib/ warp-host resolved, never from the folder the compiler was built
# in (notes/compiler_rebuilds.md, card compiler-rebuild-policy): a compiler-self.wasm built in another tree sees a word
# added to a temporary lib's os.warp, named by WARP_LIB, found beside the binary, or through the launcher. Run it
# through the queue: tests/queue.sh tests/host/test_host_library.sh
# The host and the compilers as in test_host.sh: WARP_HOST_BINARY (default target/host/warp-host, else target/release/,
# else target/debug/), compiler-self.wasm from beside WARP_LAUNCHER_RUST (default target/debug/warp).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/host-library-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler=$(dirname "$rust")/compiler-self.wasm
[ -f "$compiler" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/installed" "$stage/stub" "$stage/work" || exit 1
# lib/ copied into the stage (two places, one content: the kept values are shared), with a word no other lib/ has
cp -R "$checkout/lib" "$stage/lib" || exit 1
printf '\n%s\n' 'word_of_the_temporary_lib() := 42' >>"$stage/lib/os.warp"
cp -R "$stage/lib" "$stage/installed/lib" || exit 1
for folder in bin installed; do
	cp "$host" "$stage/$folder/warp-host" && cp "$compiler" "$stage/$folder/" || exit 1
done
cp "$checkout/packaging/bin/warp" "$stage/installed/" || exit 1
printf '#!/usr/bin/env bash\necho "ran by warp-rust"\n' >"$stage/stub/warp-rust" && chmod +x "$stage/stub/warp-rust"
# the program lies outside any lib/'s tree: only WARP_LIB or the lib/ beside the binary can give the word
printf '%s\n' 'use os' 'print word_of_the_temporary_lib()' >"$stage/work/word.warp"
cd "$stage/work" || exit 1

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
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}

built_in=$(LC_ALL=C grep -ao '/[A-Za-z0-9_./-]*/lib/compiler' "$compiler" | head -1)
expect "the compiler was built in another folder" 0 "elsewhere" -- \
	bash -c '[ -n "$0" ] && [ "$0" != "$1/lib/compiler" ] && echo elsewhere' "$built_in" "$stage"
expect "WARP_LIB names the lib/: its os.warp's new word" 0 "42" -- env WARP_LIB="$stage/lib" "$stage/bin/warp-host" run word.warp
expect "the lib/ beside warp-host: its new word" 0 "42" -- "$stage/installed/warp-host" run word.warp
expect "the lib/ beside warp-host wins over the one above the program" 0 "42" -- \
	bash -c 'mkdir -p "$1/inside" && cp word.warp "$1/inside/" && cd "$1/inside" && "$0" run word.warp' \
	"$stage/installed/warp-host" "$checkout/scratch"
expect "the launcher: the lib/ beside it, its new word" 0 "42" -- \
	env WARP_RUST="$stage/stub/warp-rust" "$stage/installed/warp" word.warp
expect "a program sees the environment's WARP_LIB, not the compiler's" 0 "none" -- \
	"$stage/installed/warp-host" run $'use os\nprint (env("WARP_LIB") ?? "none")'
rm -rf "$checkout/scratch/inside"

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi

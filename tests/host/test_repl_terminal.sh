#!/usr/bin/env bash
# The console on a terminal (card pure-repl): warp-host alone, standard input and output a pseudo-terminal
# (tests/host/terminal.py types the lines), runs lib/repl.warp; libedit's readline, called from warp, edits the line and
# keeps the history in ~/.warp_history. Run it through the queue: tests/queue.sh tests/host/test_repl_terminal.sh
# The host and compiler as tests/host/test_repl.sh finds them; the line editor's cases need libedit (libedit-dev on
# Linux: warp-host loads libedit.so)
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/repl-terminal-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$(dirname "$rust")/warp-self.wasm" ] || { echo "✗ no warp-self.wasm beside $rust: $rust rebuild builds it"; exit 1; }
command -v python3 >/dev/null || { echo "✗ no python3 for the pseudo-terminal"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/home" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$(dirname "$rust")/warp-self.wasm" "$stage/bin/" || exit 1
cd "$stage" || exit 1
library=libedit.so
[ "$(uname)" = Darwin ] && library=libedit.dylib
python3 -c "import ctypes; ctypes.CDLL('$library')" 2>/dev/null && editing=1 || editing=

run=0 failed=0
# typed <name> <status> <text the output holds>… [! <text it lacks>…] -- <typed…>: warp-host alone on a terminal
typed() {
	local name=$1 status=$2 wanted=() unwanted=() said text problems=""
	shift 2
	while [ "$1" != -- ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	shift
	said=$(HOME=$stage/home timeout 300 python3 "$checkout/tests/host/terminal.py" --quiet 3 -- "$stage/bin/warp-host" -- "$@" 2>&1)
	local got=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	for text in "${unwanted[@]}"; do [[ $said == *"$text"* ]] && problems+=" unwanted [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}

typed "warp-host alone on a terminal is the console" 0 "Interactive console" $'7\n' ! "usage" -- 'a = 7' 'a' '^D'
typed "Ctrl+D ends the console" 0 "🌀" ! "usage" -- '^D'
if [ -n "$editing" ]; then
	typed "the up arrow brings the last line back to edit" 0 "42" -- 'x = 6' '^[A * 7' '^D'
	typed "the history stays for the next console" 0 "42" -- '^[A' '' '^D'
	typed "a line goes on while its brackets are open" 0 "[1 2]" -- '[1' ' 2]' '^D'
	run=$((run + 1))
	if grep -q "x" "$stage/home/.warp_history"; then
		echo "✓ ~/.warp_history written"
	else
		failed=$((failed + 1))
		echo "✗ no ~/.warp_history"
	fi
else
	echo "– no $library: the line editor's cases skipped"
fi

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi

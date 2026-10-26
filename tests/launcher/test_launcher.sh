#!/usr/bin/env bash
# What the launcher packaging/bin/warp prints and exits with (notes/pure_wrapper.md): programs through the pure
# pipeline, other commands through warp-rust. Run it through the queue: tests/queue.sh tests/launcher/test_launcher.sh
# The Rust binary under test is WARP_LAUNCHER_RUST (default the shared warp, scripts/shared-warp.sh) with warp.wasm and
# warp-self.wasm beside it (`warp rebuild pure`, `warp rebuild self`); it and the launcher are staged together in
# scratch/launcher-test/, as installed. Refusals and failures come from a stand-in warp-rust (stub_rust below), so these
# checks hold whatever the pure pipeline covers.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
rust=${WARP_LAUNCHER_RUST:-$("$checkout"/scripts/shared-warp.sh "$checkout")}
stage=$checkout/scratch/launcher-test
hello=$checkout/samples/hello/hello.warp
HELLO='"Hello 🌍2026"'
REFUSAL='Error("outside the core: `stub` at 1:1")'
NOTICE='WARP_FALLBACK=rust: THE PURE PIPELINE REFUSED THIS PROGRAM'
export WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

for compiler in warp.wasm warp-self.wasm; do
	[ -f "$(dirname "$rust")/$compiler" ] || { echo "✗ no $compiler beside $rust: $rust rebuild builds it"; exit 1; }
done
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/elsewhere" "$stage/stub" || exit 1
ln -f "$rust" "$stage/bin/warp-rust" 2>/dev/null || cp "$rust" "$stage/bin/warp-rust" || exit 1
cp "$(dirname "$rust")"/compiler*.wasm "$checkout/packaging/bin/warp" "$stage/bin/" || exit 1
cp "$hello" "$stage/hello.warp" || exit 1
# reached through a link, as Homebrew links bin/warp: warp-rust is found beside the link's target
ln -s "$stage/bin/warp" "$stage/elsewhere/warp"
warp=$stage/elsewhere/warp

# A stand-in warp-rust: its pure pipeline (`pure|self …`, `…warp.wasm -- …`) refuses programs named refused, fails
# those named failing, exits 3 for those named exits; it says what it was given on stderr, and "ran by <lane>"
stub_rust=$stage/stub/warp-rust
cat >"$stub_rust" <<'STUB'
#!/usr/bin/env bash
echo "warp-rust $* (WARP_SELF_HOSTED=${WARP_SELF_HOSTED-unset})" >&2
case $1 in pure | self | *warp*.wasm) lane=pure ;; *) lane=rust ;; esac
if [ $lane = pure ]; then
	case $* in
		*refused*) echo 'Error("outside the core: `stub` at 1:1")'; exit 1 ;;
		*failing*) echo 'printed first'; echo 'Error("failing at 1:1")'; exit 1 ;;
		*exits*) echo 3; echo 'printed last'; exit 3 ;;
	esac
fi
echo "ran by $lane"
STUB
chmod +x "$stub_rust" && touch "$stage/stub/warp.wasm" "$stage/stub/refused.warp" || exit 1
mkdir -p "$stage/stub/lib/compiler"
stub() { WARP_RUST=$stub_rust "$warp" "$@"; }

run=0 failed=0
# expect <name> <exit status> <text the output holds>… [! <text it lacks>…] -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() unwanted=() said got lacking=
	shift 2
	while [ "$1" != -- ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	local text problems=""
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

expect "a program file runs through the pure pipeline" 0 "$HELLO" -- "$warp" "$hello"
expect "warp run" 0 "$HELLO" -- "$warp" run "$hello"
expect "--self: warp-self.wasm" 0 "$HELLO" -- "$warp" --self "$hello"
expect "WARP_COMPILER=self: warp-self.wasm" 0 "$HELLO" -- env WARP_COMPILER=self "$warp" "$hello"
expect "WARP_COMPILER=other: refused" 1 "WARP_COMPILER is pure or self" -- env WARP_COMPILER=rusty "$warp" "$hello"
expect "inline code prints its value and exits 0" 0 "42" -- "$warp" '6*7'
expect "inline code's output" 0 "inline 42" -- "$warp" 'print "inline " + 6*7'
expect "compile --wasm writes the module" 0 "bytes to $stage/hello.wasm" -- "$warp" compile --wasm "$stage/hello.warp"
expect "the compiled module runs" 0 "$HELLO" -- "$warp" "$stage/hello.wasm"
expect "other commands are warp-rust's: version" 0 "🌀 Warp" -- "$warp" version
expect "other commands are warp-rust's: help" 0 "warp pure <file.warp>" -- "$warp" --help

expect "a refused program fails loudly" 1 "$REFUSAL" ! "ran by" -- stub "$stage/stub/refused.warp"
expect "refused inline code fails loudly" 1 "$REFUSAL" ! "ran by" -- stub 'refused()'
expect "a refused compile fails loudly" 1 "$REFUSAL" ! "compiled" -- stub compile --wasm "$stage/stub/refused.warp"
expect "WARP_FALLBACK=rust: a refused program runs through Rust, said loudly" 0 "$NOTICE" "warp-rust run refused.warp" \
	"ran by rust" -- env WARP_FALLBACK=rust WARP_RUST="$stub_rust" "$warp" run refused.warp
expect "WARP_FALLBACK=rust: refused inline code runs through Rust" 0 "$NOTICE" "ran by rust" -- \
	env WARP_FALLBACK=rust WARP_RUST="$stub_rust" "$warp" 'refused()'
expect "WARP_FALLBACK=rust: a failing program is no refusal" 1 "Error(\"failing" ! "$NOTICE" ! "ran by rust" -- \
	env WARP_FALLBACK=rust WARP_RUST="$stub_rust" "$warp" failing.warp
expect "WARP_FALLBACK=rust: a program's exit status passes" 3 "printed last" ! "$NOTICE" -- \
	env WARP_FALLBACK=rust WARP_RUST="$stub_rust" "$warp" exits.warp
expect "inline code's own exit status passes" 3 "printed last" -- stub 'exits()'
expect "WARP_PIPELINE=rust: every command goes to warp-rust" 0 "warp-rust refused.warp" "ran by rust" -- \
	env WARP_PIPELINE=rust WARP_RUST="$stub_rust" "$warp" refused.warp
expect "WARP_SELF_HOSTED passes through untouched" 0 "warp-rust pure hello.warp (WARP_SELF_HOSTED=pure-only)" -- \
	env WARP_SELF_HOSTED=pure-only WARP_RUST="$stub_rust" "$warp" hello.warp
expect "--self names the self compiler, flags go along" 0 "warp-rust self --no-hints hello.warp" -- \
	stub --self --no-hints hello.warp

# warp fix and warp beautify: lib/fix/command.warp, run by warp-rust with the command word and its arguments, from the
# lib/ found as compile's is (here the nearest one above the current folder, else WARP_LIB); without it warp-rust's own
in_folder() { (cd "$1" && shift && "$@"); }
mkdir -p "$stage/stub/lib/fix" "$stage/stub/deeper/down" "$stage/older/lib/compiler" "$stage/fix" || exit 1
touch "$stage/stub/lib/fix/command.warp"
FIX_PROGRAM="run $stage/stub/lib/fix/command.warp --"
expect "warp fix runs lib/fix/command.warp" 0 "$FIX_PROGRAM fix check a.warp" "ran by rust" -- \
	in_folder "$stage/stub" stub fix check a.warp
expect "warp beautify too, flags going along" 0 "--strict run $stage/stub/lib/fix/command.warp -- beautify yes a.warp" -- \
	in_folder "$stage/stub" stub --strict beautify yes a.warp
expect "warp fix: lib/ the nearest above the current folder" 0 "$FIX_PROGRAM fix x=1" -- \
	in_folder "$stage/stub/deeper/down" stub fix x=1
expect "warp fix: WARP_LIB names lib/" 0 "$FIX_PROGRAM fix a.warp" -- \
	env WARP_LIB="$stage/stub/lib" WARP_RUST="$stub_rust" "$warp" fix a.warp
expect "warp fix: a lib/ without fix/ leaves it to warp-rust" 0 "warp-rust fix a.warp" ! "command.warp" -- \
	env WARP_LIB="$stage/older/lib" WARP_RUST="$stub_rust" "$warp" fix a.warp
expect "WARP_PIPELINE=rust: warp fix goes to warp-rust" 0 "warp-rust beautify a.warp" ! "command.warp" -- \
	in_folder "$stage/stub" env WARP_PIPELINE=rust WARP_RUST="$stub_rust" "$warp" beautify a.warp
# the real lib/fix of this checkout, by the Rust binary under test
messy() { printf 'x=1+2 // sum\nprint x\n' >"$stage/fix/messy.warp"; }
fixed_file() { "$@" && grep -qx 'x = 1 + 2 # sum' "$stage/fix/messy.warp"; }
unfixed_file() { "$@"; local status=$?; grep -qx 'x=1+2 // sum' "$stage/fix/messy.warp" && return $status; }
messy
expect "warp fix check lists the changes, exit 1, file kept" 1 "messy.warp:1: layout:" "1 to format" -- \
	unfixed_file in_folder "$checkout" "$warp" fix check "$stage/fix/messy.warp"
expect "warp beautify yes writes the fixed file" 0 "formatted $stage/fix/messy.warp" -- \
	fixed_file in_folder "$checkout" "$warp" beautify yes "$stage/fix/messy.warp"
expect "warp fix check of a fixed file: nothing to format" 0 "0 to format" -- \
	in_folder "$checkout" "$warp" fix check "$stage/fix/messy.warp"
messy
expect "warp fix shows the diff, unanswered: not applied" 0 "+x = 1 + 2 # sum" "not applied" -- \
	unfixed_file in_folder "$checkout" "$warp" fix "$stage/fix/messy.warp" </dev/null
expect "warp fix answered y applies it" 0 "apply? y/n" "1 changed" -- \
	fixed_file in_folder "$checkout" "$warp" fix "$stage/fix/messy.warp" <<<y
expect "warp beautify <code> prints it fixed" 0 "x = 1 + 2 # sum" -- in_folder "$checkout" "$warp" beautify 'x=1+2 // sum'

if [ $failed -gt 0 ]; then echo "$failed of $run failed"; exit 1; fi
echo "✓ $run tests passed"

#!/usr/bin/env bash
# warp-host's ahead-of-time compilation (card warp-host-aot, notes/warp_host.md "AOT"): `compile --aot` writes the
# machine code f.cwasm, `warp-host f.cwasm` runs it, `build` writes a standalone executable carrying it, which runs with
# WASI, std_io and C calls as warp-host does. Run it with bash test/host/test_host_aot.sh
# The host and compilers are found as in test/host/test_host.sh (WARP_HOST_BINARY, WARP_LAUNCHER_RUST, WARP_WAT2WASM);
# staged in data/host-aot-test/. Kept values and machine code go to data/host-cache/, shared with test_host.sh.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/data/host-aot-test
HELLO='"Hello 🌍2026"'
export WARP_HOST_CACHE=${WARP_HOST_CACHE:-$checkout/data/host-cache} WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$(dirname "$rust")/warp-self.wasm" ] || { echo "✗ no warp-self.wasm beside $rust: $rust rebuild self builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/elsewhere" || exit 1
cp "$host" "$stage/bin/warp-host" && cp "$(dirname "$rust")"/warp-self.wasm "$stage/bin/" || exit 1
cp "$checkout/samples/hello/hello.warp" "$stage/" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
program arguments 'use os' 'print args'
program exits 'exit(5)'
program command 'use os' 'print command("echo hi")'
program answer '6*7'

run=0 failed=0
# expect <name> <exit status> <text the output holds>… [! <text it lacks>…] -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() unwanted=() said got
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
exists() { [ -s "$1" ] || { echo "no $1"; return 1; }; }

# compile --aot and .cwasm
expect "compile --aot writes the machine code" 0 "machine code to hello.cwasm" -- "$host" compile --aot hello.warp
expect "compile --aot writes the module too" 0 -- exists hello.wasm
expect "a .cwasm runs" 0 "$HELLO" -- "$host" hello.cwasm
expect "run of a .cwasm without the compiler beside warp-host" 0 "$HELLO" \
	-- bash -c 'cp "$0" elsewhere/warp-host && cp hello.cwasm elsewhere/ && cd elsewhere && ./warp-host hello.cwasm' "$host"
expect "compile --aot names the module, the machine code goes beside it" 0 "out/answer.cwasm" \
	-- bash -c 'mkdir -p out && "$0" compile --aot answer.warp out/answer.wasm' "$host"
expect "a .cwasm gives its value and status" 42 "42" -- "$host" out/answer.cwasm
expect "a .cwasm gets the arguments after it" 0 '"b c"' \
	-- bash -c '"$0" compile --aot arguments.warp >/dev/null && "$0" arguments.cwasm a "b c"' "$host"
expect "exit(5) of a .cwasm exits 5" 5 -- bash -c '"$0" compile --aot exits.warp >/dev/null && "$0" exits.cwasm' "$host"
expect "a .cwasm runs shell commands (std_io)" 0 "hi" ! "Error" \
	-- bash -c '"$0" compile --aot command.warp >/dev/null && "$0" command.cwasm' "$host"
expect "machine code without .cwasm runs by how it starts" 0 "$HELLO" \
	-- bash -c 'cp hello.cwasm hello-machine-code && "$0" hello-machine-code' "$host"
expect "compile --aot of a module: only its machine code" 0 "machine code to hello2.cwasm" \
	-- bash -c 'cp hello.wasm hello2.wasm && "$0" compile --aot hello2.wasm' "$host"
expect "that machine code runs" 0 "$HELLO" -- "$host" hello2.cwasm
printf 'not machine code' >junk.cwasm
expect "a .cwasm that is no machine code: a clear error" 1 "junk.cwasm is no machine code of a module" -- "$host" junk.cwasm
# machine code of another wasmtime: its version in the engine section ("49", then the target's name) changed
python3 - <<'PY'
import re
code = open("hello.cwasm", "rb").read()
open("older.cwasm", "wb").write(re.sub(rb"\x02(\d\d)(?=[\x08-\x40][a-z])", lambda m: b"\x02" + str(int(m[1]) - 1).encode(), code, count=1))
PY
expect "machine code of another warp-host: a clear error" 1 "older.cwasm was compiled by another warp-host" \
	"incompatible version" "compile it again with \`warp-host compile --aot\`" -- "$host" older.cwasm
expect "compile --aot of a missing file" 1 "could not read file nosuch.warp" -- "$host" compile --aot nosuch.warp

# build: one executable, warp-host carrying the machine code
expect "build writes the executable beside the file" 0 "wrote hello" -- "$host" build hello.warp
expect "the executable runs the program" 0 "$HELLO" -- ./hello
expect "the executable runs anywhere, without the compiler or a cache" 0 "$HELLO" -- bash -c \
	'mv hello elsewhere/hello-app && cd elsewhere && rm -f warp-host && WARP_HOST_CACHE=$PWD/empty-cache ./hello-app'
expect "build -o names the executable" 0 "wrote app" -- "$host" build arguments.warp -o app
expect "the executable gets its arguments as the program's" 0 '"b c"' -- ./app a "b c"
expect "the executable does not take warp-host's commands" 0 '"--help"' ! "usage" -- ./app --help
expect "build -o before the file" 0 "wrote exits-app" -- "$host" build -o exits-app exits.warp
expect "the executable's exit(5) exits 5" 5 -- ./exits-app
expect "the executable gives the value and its status" 42 "42" -- bash -c '"$0" build answer.warp >/dev/null && ./answer' "$host"
expect "the executable runs shell commands (std_io)" 0 "hi" ! "Error" \
	-- bash -c '"$0" build command.warp >/dev/null && ./command' "$host"
expect "build of a module" 0 "wrote hello-module" -- "$host" build hello.wasm -o hello-module
expect "that executable runs" 0 "$HELLO" -- ./hello-module
expect "build over an earlier executable: the newer program" 42 "42" ! "Hello" \
	-- bash -c '"$0" build hello.warp -o twice >/dev/null && "$0" build answer.warp -o twice >/dev/null; ./twice' "$host"
expect "build never writes over its source" 1 "build would write the executable over" \
	-- bash -c 'cp hello.warp script && "$0" build script' "$host"
expect "--help names build and compile --aot" 0 "build <file.warp" "compile [--aot]" -- "$host" --help
if [ "$(uname)" = Darwin ]; then
	expect "macOS: the executable's signature verifies strictly" 0 -- codesign --verify --strict ./app
fi

# C calls in machine code and executables: the warp.c_functions table travels in front of the machine code
assembler=${WARP_WAT2WASM:-$checkout/target/release/wat2wasm}
[ -x "$assembler" ] || assembler=$checkout/target/host/wat2wasm
[ -x "$assembler" ] || assembler=$checkout/target/debug/wat2wasm
[ -x "$assembler" ] || assembler=$(command -v wat2wasm)
if [ -x "$assembler" ]; then
	wat() { # wat <name> <module text>: <name>.wat in the stage, assembled into <name>.wasm
		printf '%s\n' "$2" >"$stage/$1.wat"
		if [ "$(basename "$assembler")" = wat2wasm ] && "$assembler" --help 2>&1 | grep -q -- --enable-all; then
			"$assembler" --enable-all "$stage/$1.wat" -o "$stage/$1.wasm"
		else
			"$assembler" "$stage/$1.wat" "$stage/$1.wasm"
		fi || echo "✗ wat2wasm could not assemble $1.wat"
	}
	NODE='(rec (type $Node (struct (field i64) (field anyref) (field (ref null $Node))))) (type $Text (array (mut i8)))'
	wat strlen "(module $NODE (import \"c\" \"strlen\" (func \$strlen (param anyref) (result i64)))
		(memory (export \"memory\") 1) (data \$hello \"hello\")
		(func (export \"main\") (result i64) (call \$strlen (struct.new \$Node (i64.const 3)
			(array.new_data \$Text \$hello (i32.const 0) (i32.const 5)) (ref.null \$Node))))
		(@custom \"warp.c_functions\" \"c\09c\09strlen\09ptr\09i64\09number\09\09\09\"))"
	wat hypot '(module (import "m" "hypot" (func $hypot (param f64 f64) (result f64)))
		(func (export "main") (result f64) (call $hypot (f64.const 3) (f64.const 4))))'
	expect "C: strlen(\"hello\") from machine code, its table carried" 5 "5" \
		-- bash -c '"$0" compile --aot strlen.wasm >/dev/null && "$0" strlen.cwasm' "$host"
	expect "C: hypot(3, 4) by dlsym from machine code" 0 "5.0" \
		-- bash -c '"$0" compile --aot hypot.wasm >/dev/null && "$0" hypot.cwasm' "$host"
	expect "C: strlen(\"hello\") in an executable" 5 "5" \
		-- bash -c '"$0" build strlen.wasm -o strlen-app >/dev/null && ./strlen-app' "$host"
else
	echo "✗ no wat2wasm: cargo build --release -p warp-runtime --features wat --bin wat2wasm"
	failed=$((failed + 1))
fi

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi

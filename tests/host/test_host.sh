#!/usr/bin/env bash
# What warp-host prints and exits with (notes/warp_host.md): pure programs compiled by compiler-self.wasm and run with
# WASI and shell commands, and the launcher packaging/bin/warp handing programs to a warp-host beside it. Run it through
# the queue: tests/queue.sh tests/host/test_host.sh
# The host under test is WARP_HOST_BINARY (default the checkout's target/release/warp-host, else target/debug/: `cargo
# build --release -p warp-runtime --features host --bin warp-host`), with compiler-self.wasm and compiler.wasm from
# beside WARP_LAUNCHER_RUST (default target/debug/warp, after `warp rebuild`); staged in scratch/host-test/ as installed.
# Kept values and machine code go to scratch/host-cache/, kept between runs: the first run computes them (~30 s).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/release/warp-host}
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/host-test
HELLO='"Hello 🌍2026"'
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --release -p warp-runtime --features host --bin warp-host"; exit 1; }
for compiler in compiler.wasm compiler-self.wasm; do
	[ -f "$(dirname "$rust")/$compiler" ] || { echo "✗ no $compiler beside $rust: $rust rebuild builds it"; exit 1; }
done
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/stub" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$(dirname "$rust")"/compiler*.wasm "$checkout/packaging/bin/warp" "$stage/bin/" || exit 1
cp "$checkout/samples/hello/hello.warp" "$stage/" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
program command 'use os' 'print command("echo hi")'
program failing 'use os' 'command("exit 3")'
program caught 'use os' 'try command("exit 3") catch e { print "caught " + e }'
program ran 'use os' 'r = run(["sh", "-c", "echo o; echo e >&2; exit 3"])' 'print "status " + r.status' \
	'print "out " + r.out' 'print "err " + r.err'
program refused 'x = eval("1+2")'
program arguments 'use os' 'print args'
program exits 'exit(5)'

# A stand-in warp-rust: says it ran, and by which lane
cat >"$stage/stub/warp-rust" <<'STUB'
#!/usr/bin/env bash
case $1 in pure | self | *compiler*.wasm) echo "ran by warp-rust pure" ;; *) echo "ran by warp-rust" ;; esac
STUB
chmod +x "$stage/stub/warp-rust"
launcher() { WARP_RUST=$stage/stub/warp-rust "$stage/bin/warp" "$@"; }

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

expect "print works" 0 "$HELLO" -- "$host" run hello.warp
expect "a .warp file runs without run" 0 "$HELLO" -- "$host" hello.warp
expect "inline code: its value, an Int its status" 42 "42" -- "$host" run '6*7'
expect "command(\"echo hi\") gives hi" 0 "hi" ! "Error" -- "$host" run command.warp
expect "command(\"exit 3\") is an Error" 1 'Error("os.command: sh -c exit 3 failed (exit status: 3)' -- "$host" run failing.warp
expect "a command's Error is caught" 0 "caught os.command: sh -c exit 3 failed" -- "$host" run caught.warp
expect "run gives status, out and err" 0 "status 3" "out o" "err e" -- "$host" run ran.warp
expect "a refusal stays a refusal" 1 'Error("outside the core: ' -- "$host" run refused.warp
expect "a refusal is the only output line" 1 ! "more" -- bash -c '[ "$("$0" run refused.warp | wc -l)" -eq 1 ] || echo more; exit 1' "$host"
expect "the arguments after the file" 0 "\"b c\"" -- "$host" run arguments.warp a "b c"
expect "exit(5) exits 5" 5 -- "$host" run exits.warp
expect "compile writes the module" 0 -- "$host" compile hello.warp hello.wasm
expect "the module runs" 0 "$HELLO" -- "$host" hello.wasm
expect "--compiler full: compiler.wasm" 0 "$HELLO" -- "$host" --compiler full run hello.warp
expect "no program: usage" 2 "usage: warp-host" -- "$host"
expect "the launcher runs a program by warp-host beside it" 0 "$HELLO" ! "ran by" -- launcher hello.warp
expect "the launcher's warp run" 0 "hi" ! "ran by" -- launcher run command.warp
expect "the launcher's inline code exits 0" 0 "42" ! "ran by" -- launcher '6*7'
expect "the launcher's compile --wasm" 0 "compiled" ! "ran by" -- launcher compile --wasm hello.warp
expect "the launcher: a refusal fails loudly" 1 'Error("outside the core: ' ! "ran by" -- launcher refused.warp
expect "the launcher: WARP_FALLBACK=rust runs a refusal through Rust" 0 "ran by warp-rust" \
	-- env WARP_FALLBACK=rust WARP_RUST="$stage/stub/warp-rust" "$stage/bin/warp" refused.warp
expect "the launcher: WARP_HOST=none is warp-rust's pure lane" 0 "ran by warp-rust pure" -- env WARP_HOST=none \
	WARP_RUST="$stage/stub/warp-rust" "$stage/bin/warp" hello.warp
expect "the launcher: other commands stay warp-rust's" 0 "ran by warp-rust" ! "pure" -- launcher version

# C calls (M2): hand-written modules importing C functions, as the pure emitter will (card pure-c-imports); a text
# is a warp text node, its warp.c_functions line says how each crosses (c_signatures.rs)
wat() { # wat <name> <module text>: <name>.wat in the stage
	printf '%s\n' "$2" >"$stage/$1.wat"
}
NODE='(rec (type $Node (struct (field i64) (field anyref) (field (ref null $Node))))) (type $Text (array (mut i8)))'
text() { echo "(struct.new \$Node (i64.const 3) (array.new_data \$Text $1 (i32.const 0) (i32.const $2)) (ref.null \$Node))"; }
wat strlen "(module $NODE (import \"c\" \"strlen\" (func \$strlen (param anyref) (result i64)))
	(memory (export \"memory\") 1) (data \$hello \"hello\")
	(func (export \"main\") (result i64) (call \$strlen $(text '$hello' 5)))
	(@custom \"warp.c_functions\" \"c\09c\09strlen\09ptr\09i64\09number\09\09\09\"))"
wat sqrt '(module (import "m" "sqrt" (func $sqrt (param f64) (result f64)))
	(func (export "main") (result f64) (call $sqrt (f64.const 16))))'
wat hypot '(module (import "m" "hypot" (func $hypot (param f64 f64) (result f64)))
	(func (export "main") (result f64) (call $hypot (f64.const 3) (f64.const 4))))'
line() { printf 'sqlite3\\09sqlite3\\09%s\\09%s\\09i32\\09%s\\09%s\\09\\09' "$@"; }
wat sqlite "(module $NODE
	(import \"sqlite3\" \"sqlite3_open\" (func \$open (param anyref) (result i32)))
	(import \"sqlite3\" \"sqlite3_prepare_v2\" (func \$prepare (param i32 anyref i32) (result i32)))
	(import \"sqlite3\" \"sqlite3_step\" (func \$step (param i32) (result i32)))
	(import \"sqlite3\" \"sqlite3_column_int\" (func \$column_int (param i32 i32) (result i32)))
	(memory (export \"memory\") 1) (data \$name \":memory:\") (data \$sql \"select 1+2\")
	(func (export \"main\") (result i64) (local \$db i32) (local \$stmt i32)
		(local.set \$db (call \$open $(text '$name' 8)))
		(local.set \$stmt (call \$prepare (local.get \$db) $(text '$sql' 10) (i32.const -1)))
		(drop (call \$step (local.get \$stmt)))
		(i64.extend_i32_s (call \$column_int (local.get \$stmt) (i32.const 0))))
	(@custom \"warp.c_functions\" \"$(line sqlite3_open ptr,out handle sqlite3)\\0a$(line sqlite3_prepare_v2 \
		handle,ptr,i32,out,out handle sqlite3_stmt)\\0a$(line sqlite3_step handle number '')\\0a$(line \
		sqlite3_column_int handle,i32 number '')\"))"
wat nolib '(module (import "nosuchlib" "nosuch_function" (func $f (result i32))) (func (export "main") (result i32) (call $f)))'
wat nosymbol '(module (import "c" "no_such_c_function" (func $f (result i32))) (func (export "main") (result i32) (call $f)))'

expect "C: strlen(\"hello\") gives 5" 5 "5" -- "$host" strlen.wat
expect "C: sqrt(16) gives 4.0" 0 "4.0" -- "$host" sqrt.wat
expect "C: hypot(3, 4) from libm by dlsym gives 5.0" 0 "5.0" -- "$host" hypot.wat
expect "C: sqlite3 :memory: select 1+2 gives 3" 3 "3" -- "$host" sqlite.wat
expect "C: a missing library is a clear error" 1 "the C library nosuchlib of the import nosuchlib.nosuch_function is not found (tried" \
	-- "$host" nolib.wat
expect "C: a missing symbol is a clear error" 1 "the C library c has no function no_such_c_function" -- "$host" nosymbol.wat

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi

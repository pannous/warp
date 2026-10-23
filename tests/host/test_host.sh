#!/usr/bin/env bash
# What warp-host prints and exits with (notes/warp_host.md): pure programs compiled by compiler-self.wasm and run with
# WASI and shell commands, and the launcher packaging/bin/warp handing programs to a warp-host beside it. Run it through
# the queue: tests/queue.sh tests/host/test_host.sh
# The host under test is WARP_HOST_BINARY (default the checkout's target/host/warp-host, else target/release/, else
# target/debug/: `cargo build --profile host -p warp-runtime --features host --bin warp-host`), with compiler-self.wasm
# and compiler.wasm from beside WARP_LAUNCHER_RUST (default the shared warp, scripts/shared-warp.sh); staged in
# scratch/host-test/ as installed.
# Kept values and machine code go to scratch/host-cache/, kept between runs: the first run computes them (~30 s).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$("$checkout"/scripts/shared-warp.sh "$checkout")}
stage=$checkout/scratch/host-test
HELLO='"Hello 🌍2026"'
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
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
# no command means run (as warp): a file, code, their arguments; a module by its name or its magic bytes
expect "no command: a .warp file runs" 0 "$HELLO" -- "$host" hello.warp
expect "no command: inline code, its value its status" 42 "42" -- "$host" '6*7'
expect "no command: inline code gets the words after it" 0 "['a' \"b c\"]" -- "$host" $'use os\nprint args' a "b c"
expect "no command: the arguments after the file" 0 "\"b c\"" -- "$host" arguments.warp a "b c"
expect "no command: --compiler full before code" 42 "42" -- "$host" --compiler full '6*7'
expect "no command: a .wasm file still runs as a module" 0 "$HELLO" -- "$host" hello.wasm
expect "no command: a module without .wasm runs by its magic bytes" 0 "$HELLO" \
	-- bash -c 'cp hello.wasm hello-module && "$0" hello-module' "$host"
expect "a missing .warp file is an error, not code" 1 "could not read file nosuch.warp" -- "$host" nosuch.warp
expect "run of a missing .warp file is that error too" 1 "could not read file nosuch.warp" -- "$host" run nosuch.warp
expect "--help: usage, status 0" 0 "usage: warp-host" "no command means run" -- "$host" --help
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
# assembled to <name>.wasm by wat2wasm, as warp-host reads WAT text only with the feature wat: WARP_WAT2WASM, else the
# checkout's (`cargo build --release -p warp-runtime --features wat --bin wat2wasm`), else one on PATH (wabt)
assembler=${WARP_WAT2WASM:-$checkout/target/release/wat2wasm}
[ -x "$assembler" ] || assembler=$checkout/target/host/wat2wasm
[ -x "$assembler" ] || assembler=$checkout/target/debug/wat2wasm
[ -x "$assembler" ] || assembler=$(command -v wat2wasm)
[ -x "$assembler" ] || { echo "✗ no wat2wasm: cargo build --release -p warp-runtime --features wat --bin wat2wasm"; exit 1; }
wat() { # wat <name> <module text>: <name>.wat in the stage, assembled into <name>.wasm
	printf '%s\n' "$2" >"$stage/$1.wat"
	if [ "$(basename "$assembler")" = wat2wasm ] && "$assembler" --help 2>&1 | grep -q -- --enable-all; then
		"$assembler" --enable-all "$stage/$1.wat" -o "$stage/$1.wasm"
	else
		"$assembler" "$stage/$1.wat" "$stage/$1.wasm"
	fi || echo "✗ wat2wasm could not assemble $1.wat"
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

expect "C: strlen(\"hello\") gives 5" 5 "5" -- "$host" strlen.wasm
expect "C: sqrt(16) gives 4.0" 0 "4.0" -- "$host" sqrt.wasm
expect "C: hypot(3, 4) from libm by dlsym gives 5.0" 0 "5.0" -- "$host" hypot.wasm
expect "C: sqlite3 :memory: select 1+2 gives 3" 3 "3" -- "$host" sqlite.wasm
expect "C: a missing library is a clear error" 1 "the C library nosuchlib of the import nosuchlib.nosuch_function is not found (tried" \
	-- "$host" nolib.wasm
expect "C: a missing symbol is a clear error" 1 "the C library c has no function no_such_c_function" -- "$host" nosymbol.wasm

# run_wasm (card warp-host-run-wasm): a module's bytes run inside the program's warp-host, with the same imports and
# stdout; compile_wasm gives a program's bytes
wat answer '(module (func (export "main") (result i64) (i64.const 42)))'
wat trapping '(module (func (export "main") (result i64) unreachable))'
wat printing '(module (import "wasi_snapshot_preview1" "fd_write" (func $w (param i32 i32 i32 i32) (result i32)))
	(memory (export "memory") 1) (data (i32.const 16) "printed by the module\n")
	(func (export "main") (result i64) (i32.store (i32.const 0) (i32.const 16)) (i32.store (i32.const 4) (i32.const 22))
		(drop (call $w (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8))) (i64.const 7)))'
program run_answer 'use os' 'use file' 'print run_wasm(read_bytes("answer.wasm"))'
program run_twice 'use os' 'use file' 'b = read_bytes("answer.wasm")' 'print "first " + run_wasm(b)' \
	'print "second " + run_wasm(b)'
program run_trapping 'use os' 'use file' 'try run_wasm(read_bytes("trapping.wasm")) catch e { print "caught " + e }'
program run_printing 'use os' 'use file' 'x = run_wasm(read_bytes("printing.wasm"))' 'print "then " + x'
program run_compiled 'use os' 'print run_wasm(compile_wasm("6*7"))'
# the module's machine code, cached: an empty cache with the suite's as the shipped one, and a module of its own
wat cached '(module (func (export "main") (result i64) (i64.const 43)))'
program run_cached 'use os' 'use file' 'print run_wasm(read_bytes("cached.wasm"))'
fresh=$stage/run-wasm-cache
rm -rf "$fresh"
cached() { # cached: run_cached.warp's output, then how much machine code the empty cache holds
	WARP_HOST_SHIPPED_CACHE=$WARP_HOST_CACHE WARP_HOST_CACHE=$fresh "$host" run run_cached.warp
	echo "machine code: $(find "$fresh" -name '*.cwasm' 2>/dev/null | wc -l | tr -d ' ')"
}
expect "run_wasm of a module giving 42" 0 "42" -- "$host" run run_answer.warp
expect "run_wasm: the second run of the same bytes" 0 "first 42" "second 42" -- "$host" run run_twice.warp
expect "run_wasm: a first run makes the module's machine code" 0 "43" "machine code: 1" -- cached
expect "run_wasm: a warm run takes it from the cache, making none" 0 "43" "machine code: 1" -- cached
expect "run_wasm: a trap is an Error" 0 "caught os.run_wasm: wasm trap: wasm \`unreachable\` instruction executed" \
	-- "$host" run run_trapping.warp
expect "run_wasm: the module prints to the same stdout" 0 "printed by the module" "then 7" -- "$host" run run_printing.warp
expect "run_wasm(compile_wasm(\"6*7\")) evaluates code" 0 "42" -- "$host" run run_compiled.warp
program run_with_args 'use os' 'print run_wasm(compile_wasm("use os; args"), ["x", "y z"])' \
	'print run_wasm(compile_wasm("use os; args"))'
expect "run_wasm(bytes, args): the module sees its own arguments" 0 "['x' \"y z\"]" "['p' 'q']" ! "\"'x'\"" \
	-- "$host" run run_with_args.warp p q
# an uncaught Warp error of the module is its Error, passed through; a trap is a failure of its own (run_trapping)
program run_raised 'use os' 'try run_wasm(compile_wasm("raise \"bad thing\"")) catch e { print "caught " + e }'
expect "run_wasm: the module's uncaught Error passes through, no trap" 0 "caught bad thing" ! "os.run_wasm" ! "wasm trap" \
	-- "$host" run run_raised.warp

# use wasm (card warp-host-wasm-load): other modules and components loaded into the program and called; imports naming
# a .wasm file linked to that module. Components are assembled from WAT by the checkout's wat2wasm (the wat crate)
wat calc '(module (global $count (mut i32) (i32.const 0))
	(func (export "twice") (param i64) (result i64) (i64.mul (local.get 0) (i64.const 2)))
	(func (export "half") (param f64) (result f64) (f64.div (local.get 0) (f64.const 2)))
	(func (export "add32") (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1)))
	(func (export "divmod") (param i64 i64) (result i64 i64)
		(i64.div_s (local.get 0) (local.get 1)) (i64.rem_s (local.get 0) (local.get 1)))
	(func (export "tick") (result i32) (global.set $count (i32.add (global.get $count) (i32.const 1))) (global.get $count))
	(func (export "boom") unreachable))'
mkdir -p "$stage/deps"
wat deps/counter '(module (global $n (mut i64) (i64.const 0)) (func (export "count_up") (param i64) (result i64)
	(global.set $n (i64.add (global.get $n) (local.get 0))) (global.get $n)))'
wat deps/doubler '(module (import "deps/counter.wasm" "count_up" (func $count_up (param i64) (result i64)))
	(func (export "twice") (param i64) (result i64) (i64.mul (local.get 0) (i64.const 2)))
	(func (export "count_twice") (param i64) (result i64) (drop (call $count_up (local.get 0))) (call $count_up (local.get 0))))'
wat importer '(module (import "deps/doubler.wasm" "twice" (func $twice (param i64) (result i64)))
	(import "deps/doubler.wasm" "count_twice" (func $count_twice (param i64) (result i64)))
	(import "deps/counter.wasm" "count_up" (func $count_up (param i64) (result i64)))
	(func (export "main") (result i64) (drop (call $count_twice (i64.const 5)))
		(i64.add (call $twice (i64.const 21)) (call $count_up (i64.const 0)))))'
wat bare '(module (import "calc" "add32" (func $add (param i32 i32) (result i32)))
	(func (export "main") (result i32) (call $add (i32.const 40) (i32.const 2))))'
wat missing_module '(module (import "nosuch.wasm" "f" (func $f (result i32))) (func (export "main") (result i32) (call $f)))'
wat cycle_a '(module (import "cycle_b.wasm" "g" (func (result i32))) (func (export "f") (result i32) (i32.const 1)))'
wat cycle_b '(module (import "cycle_a.wasm" "f" (func (result i32))) (func (export "g") (result i32) (i32.const 2)))'
# a component: add(s32, s32), manhattan(record point), shout(string) -> string, sum(list<s64>)
wat shouter '(component
	(core module $m (memory (export "memory") 1) (global $next (mut i32) (i32.const 1024))
		(func (export "realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
			(local.set $at (i32.and (i32.add (global.get $next) (i32.sub (local.get 2) (i32.const 1)))
				(i32.sub (i32.const 0) (local.get 2))))
			(global.set $next (i32.add (local.get $at) (local.get 3))) (local.get $at))
		(func (export "add") (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1)))
		(func (export "manhattan") (param i32 i32) (result i32)
			(i32.add (select (local.get 0) (i32.sub (i32.const 0) (local.get 0)) (i32.ge_s (local.get 0) (i32.const 0)))
				(select (local.get 1) (i32.sub (i32.const 0) (local.get 1)) (i32.ge_s (local.get 1) (i32.const 0)))))
		(func (export "shout") (param $p i32) (param $n i32) (result i32) (local $i i32) (local $out i32) (local $c i32)
			(local $ret i32) (local.set $out (global.get $next))
			(global.set $next (i32.add (global.get $next) (i32.add (local.get $n) (i32.const 8))))
			(block $done (loop $each (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
				(local.set $c (i32.load8_u (i32.add (local.get $p) (local.get $i))))
				(if (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
					(then (local.set $c (i32.sub (local.get $c) (i32.const 32)))))
				(i32.store8 (i32.add (local.get $out) (local.get $i)) (local.get $c))
				(local.set $i (i32.add (local.get $i) (i32.const 1))) (br $each)))
			(local.set $ret (i32.and (i32.add (i32.add (local.get $out) (local.get $n)) (i32.const 3)) (i32.const -4)))
			(i32.store (local.get $ret) (local.get $out)) (i32.store offset=4 (local.get $ret) (local.get $n)) (local.get $ret))
		(func (export "sum") (param $p i32) (param $n i32) (result i64) (local $i i32) (local $s i64)
			(block $done (loop $each (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
				(local.set $s (i64.add (local.get $s) (i64.load (i32.add (local.get $p) (i32.shl (local.get $i) (i32.const 3))))))
				(local.set $i (i32.add (local.get $i) (i32.const 1))) (br $each)))
			(local.get $s)))
	(core instance $i (instantiate $m))
	(type $point (record (field "x" s32) (field "y" s32)))
	(export $point_type "point" (type $point))
	(func (export "add") (param "a" s32) (param "b" s32) (result s32) (canon lift (core func $i "add")))
	(func (export "manhattan") (param "p" $point_type) (result s32) (canon lift (core func $i "manhattan")))
	(func (export "shout") (param "text" string) (result string) (canon lift (core func $i "shout")
		(memory (core memory $i "memory")) (realloc (core func $i "realloc")) string-encoding=utf8))
	(func (export "sum") (param "xs" (list s64)) (result s64) (canon lift (core func $i "sum")
		(memory (core memory $i "memory")) (realloc (core func $i "realloc")))))'
cp "$checkout/tests/fixtures/components/rust_demo.wasm" "$stage/" || echo "✗ no tests/fixtures/components/rust_demo.wasm"
program wasm_calls 'use wasm' 'm = load_wasm("calc.wasm")' 'print "twice " + wasm_call(m, "twice", [21])' \
	'print "half " + wasm_call(m, "half", [5.0])' 'print "add32 " + wasm_call("calc.wasm", "add32", [2, 3])' \
	'print "divmod " + wasm_call(m, "divmod", [17, 5])'
program wasm_handle 'use wasm' 'print load_wasm("calc.wasm")'
program wasm_state 'use wasm' 'm = load_wasm("calc.wasm")' 'wasm_call(m, "tick", [])' 'print wasm_call("calc.wasm", "tick", [])'
program wasm_exports 'use wasm' 'print wasm_exports(load_wasm("calc.wasm"))'
program wasm_trap 'use wasm' 'try wasm_call("calc.wasm", "boom", []) catch e { print "caught " + e }'
program wasm_no_function 'use wasm' 'print wasm_call("calc.wasm", "nope", [])'
program wasm_no_file 'use wasm' 'print load_wasm("nosuch.wasm")'
program wasm_warp_value 'use os' 'use wasm' 'm = load_wasm(compile_wasm("[1, 2, \"three\", {a: 4}]"))' \
	'x = wasm_call(m, "main", [])' 'print x' 'print x#3'
program component_calls 'use wasm' 'c = load_wasm("shouter.wasm")' 'print "add " + wasm_call(c, "add", [40, 2])' \
	'print "manhattan " + wasm_call(c, "manhattan", [{x: 3, y: -4}])' 'print wasm_call(c, "shout", ["hello wasm"])' \
	'print "sum " + wasm_call(c, "sum", [[1, 2, 3, 4]])'
program component_exports 'use wasm' 'print wasm_exports(load_wasm("shouter.wasm"))'
program component_wrong 'use wasm' 'print wasm_call("shouter.wasm", "add", [3000000000, 1])' \
	'print wasm_call("shouter.wasm", "add", [1])'
program component_rust 'use wasm' 'c = load_wasm("rust_demo.wasm")' 'print "fib " + wasm_call(c, "fib", [10])' \
	'print wasm_call(c, "words", ["hello big world"])' 'print wasm_call(c, "stats_of", ["hello big world"])' \
	'print "sides " + wasm_call(c, "sides", ["square"])' 'print "sqrt " + wasm_call(c, "text_tools.checked_sqrt", [16.0])' \
	'try wasm_call(c, "checked_sqrt", [-1.0]) catch e { print "caught " + e }'
expect "wasm: load_wasm and wasm_call by the export's types" 0 "twice 42" "half 2.5" "add32 5" "divmod [3 2]" \
	-- "$host" run wasm_calls.warp
expect "wasm: a module's handle" 0 '{wasm:1 path:"calc.wasm" kind:"module"}' -- "$host" run wasm_handle.warp
expect "wasm: a path is loaded once per run, its state lasts" 0 "2" -- "$host" run wasm_state.warp
expect "wasm: wasm_exports gives names and types" 0 'twice:"func(i64) -> i64"' 'divmod:"func(i64, i64) -> (i64, i64)"' \
	-- "$host" run wasm_exports.warp
expect "wasm: a trap is a catchable Error" 0 "caught wasm.call: calc.wasm's boom failed: wasm trap: wasm \`unreachable\`" \
	-- "$host" run wasm_trap.warp
expect "wasm: a missing function names those there are" 0 "calc.wasm has no function nope (it has twice, half" \
	-- "$host" run wasm_no_function.warp
expect "wasm: a missing file is an Error" 0 "wasm.load: no module nosuch.wasm" -- "$host" run wasm_no_file.warp
expect "wasm: a warp module's value crosses into the program" 0 '[1 2 "three" {a:4}]' "three" \
	-- "$host" run wasm_warp_value.warp
expect "wasm imports: a module's imports naming .wasm files, one instance each" 52 "52" -- "$host" importer.wasm
expect "wasm imports: a bare name with name.wasm beside" 42 "42" -- "$host" bare.wasm
expect "wasm imports: a missing module is a clear error" 1 "the import module nosuch.wasm: no module nosuch.wasm" \
	-- "$host" missing_module.wasm
expect "wasm imports: a cycle is a clear error" 1 "the modules import each other in a cycle: cycle_b.wasm → cycle_a.wasm → cycle_b.wasm" \
	-- "$host" cycle_a.wasm
expect "component: scalars, a record, a string and a list by WIT types" 0 "add 42" "manhattan 7" "HELLO WASM" "sum 10" \
	-- "$host" run component_calls.warp
expect "component: wasm_exports gives WIT types" 0 'manhattan:"func(p: record { x: s32, y: s32 }) -> s32"' \
	'shout:"func(text: string) -> string"' -- "$host" run component_exports.warp
expect "component: a wrong argument is an Error naming the parameter" 0 "add(a): 3000000000 does not fit a s32" \
	"add takes 2 arguments, got 1" -- "$host" run component_wrong.warp
expect "component: rust_demo's lists, records, enums and results" 0 "fib 55" '["hello" "big" "world"]' \
	"{words:3 letters:13}" "sides 4" "sqrt 4" "caught wasm.call: no real square root of -1" -- "$host" run component_rust.warp

# the Unix commands of lib/shell.warp (card unix-words), file.list and file.glob: each program in a fresh folder below
# the stage (work/), its own HOME beside it (home/, where rm's Trash goes), removed after
shell_in() { # shell_in [run] <code>: `use shell` and the code, run by warp-host (bare, or with run) in a fresh folder
	local folder status command=()
	[ "$1" = run ] && command=(run) && shift
	folder=$(mktemp -d "$stage/shell.XXXXXX") || return 1
	(mkdir "$folder/home" "$folder/work" && cd "$folder/work" && HOME=$folder/home "$host" "${command[@]}" "use shell"$'\n'"$1")
	status=$?
	rm -rf "$folder"
	return $status
}
expect "shell: ls gives the names, sorted, without hidden ones" 0 '["alpha.txt" "beta.txt" "sub"]' ! ".hidden" \
	-- shell_in run $'touch("beta.txt")\ntouch("alpha.txt")\ntouch(".hidden")\nmkdir("sub")\nprint ls()'
expect "shell: ls of a folder, bare warp-host" 0 '["inner.txt"]' -- shell_in $'mkdir("d")\ntouch("d/inner.txt")\nprint ls("d")'
expect "shell: pwd is the working directory" 0 "same yes" -- shell_in run $'print "same " + (pwd() == command("pwd"))'
expect "shell: cd gives the new folder, where relative paths and commands go" 0 "/sub/deeper" '["here.txt"]' \
	"pwd /" -- shell_in run $'mkdir("sub/deeper")\nprint cd("sub/deeper")\ntouch("here.txt")\nprint ls(".")\nprint "pwd " + command("pwd")'
expect "shell: cd(\"/tmp\") and a command after it" 0 "/tmp" -- shell_in run $'cd("/tmp")\nprint command("pwd")'
expect "shell: cd ~ is the home folder" 0 "/home" -- shell_in run $'print cd("~")'
expect "shell: cd of a missing folder is an Error" 0 "caught os.cd: nosuch: No such file or directory" \
	-- shell_in run $'try cd("nosuch") catch e { print "caught " + e }'
expect "shell: mkdir, write and cat round trip" 0 "one" "two" -- shell_in run \
	$'mkdir("a/b/c")\nwrite("a/b/c/notes.txt", "one\\ntwo\\n")\nprint cat("a/b/c/notes.txt")'
expect "shell: ~ in a path is the home folder" 0 "at home yes" '["x.txt"]' -- shell_in run \
	$'write("~/x.txt", "at home")\nprint "at home " + exists("~/x.txt")\nprint ls("~")'
expect "shell: touch makes an empty file, keeps a full one" 0 "empty 0" "kept 4" -- shell_in run \
	$'touch("new.txt")\nprint "empty " + file_size("new.txt")\nwrite("full.txt", "full")\ntouch("full.txt")\nprint "kept " + file_size("full.txt")'
expect "shell: cp into a folder keeps the name, cp to a file copies" 0 '["x.txt"]' "copy x" -- shell_in run \
	$'write("x.txt", "x")\nmkdir("into")\ncp("x.txt", "into")\nprint ls("into")\ncp("x.txt", "y.txt")\nprint "copy " + cat("y.txt")'
expect "shell: mv renames, and moves into a folder" 0 '["b.txt" "dir"]' '["c.txt"]' -- shell_in run \
	$'write("a.txt", "a")\nmv("a.txt", "b.txt")\nmkdir("dir")\nwrite("c.txt", "c")\nmv("c.txt", "dir")\nprint ls()\nprint ls("dir")'
expect "shell: rm moves a file to the Trash" 0 "gone no" "trashed gone.txt" -- shell_in run \
	$'write("gone.txt", "bye")\nrm("gone.txt")\nprint "gone " + exists("gone.txt")\nprint "trashed " + command("ls ~/.Trash 2>/dev/null || ls ~/.local/share/Trash/files")'
expect "shell: rm removes an empty folder" 0 "left no" -- shell_in run $'mkdir("empty")\nrm("empty")\nprint "left " + exists("empty")'
expect "shell: rm refuses a full folder" 0 "caught file.remove: rm removes a folder only when empty: full has 1 entries" \
	-- shell_in run $'mkdir("full")\ntouch("full/f.txt")\ntry rm("full") catch e { print "caught " + e }'
expect "shell: rm refuses a pattern" 0 "caught file.remove: rm takes one path, no pattern" \
	-- shell_in run $'try rm("*.txt") catch e { print "caught " + e }'
expect "shell: rm refuses the working directory" 0 "caught file.remove: rm never removes" \
	-- shell_in run $'try rm(".") catch e { print "caught " + e }'
expect "shell: list_files of a missing folder is an Error" 0 "caught file.list: nosuch: No such file or directory" \
	-- shell_in run $'try list_files("nosuch") catch e { print "caught " + e }'
expect "shell: head, tail and wc" 0 '["l1" "l2 two"]' '["l3"]' "{lines:3 words:4 bytes:13}" -- shell_in run \
	$'write("t.txt", "l1\\nl2 two\\nl3\\n")\nprint head("t.txt", 2)\nprint tail("t.txt", 1)\nprint wc("t.txt")'
expect "shell: find_files, glob and file_size" 0 '["./src/a.warp" "./src/deep/b.warp"]' '["src/a.warp"]' "size 5" \
	-- shell_in run $'mkdir("src/deep")\nwrite("src/a.warp", "ab")\nwrite("src/deep/b.warp", "cde")\ntouch("src/c.txt")\nprint find_files(".", "*.warp")\nprint glob("src/*.warp")\nprint "size " + file_size("src")'
expect "shell: info of a file" 0 "size:3" 'kind:"file"' "modified:" -- shell_in run $'write("i.txt", "abc")\nprint info("i.txt")'
expect "shell: which finds sh, ø for no program" 0 "/sh" "none ø" -- shell_in run \
	$'print which("sh")\nprint "none " + which("no-such-program-xyz")'

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi

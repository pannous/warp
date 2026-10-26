#!/usr/bin/env bash
# What warp-host prints and exits with (notes/warp_host.md): pure programs compiled by warp-self.wasm and run with
# WASI and shell commands, and the launcher packaging/bin/warp handing programs to a warp-host beside it. Run it through
# the queue: tests/queue.sh tests/host/test_host.sh
# The host under test is WARP_HOST_BINARY (default the checkout's target/host/warp-host, else target/release/, else
# target/debug/: `cargo build --profile host -p warp-runtime --features host --bin warp-host`), with warp-self.wasm
# and warp.wasm from beside WARP_LAUNCHER_RUST (default target/debug/warp, after `warp rebuild`); staged in
# scratch/host-test/ as installed.
# Kept values and machine code go to scratch/host-cache/, kept between runs: the first run computes them (~30 s).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/host-test
HELLO='"Hello 🌍2026"'
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
for compiler in warp.wasm warp-self.wasm; do
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
case $1 in pure | self | *warp*.wasm) echo "ran by warp-rust pure" ;; *) echo "ran by warp-rust" ;; esac
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
expect "--compiler full: warp.wasm" 0 "$HELLO" -- "$host" --compiler full run hello.warp
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

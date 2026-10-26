#!/usr/bin/env bash
# Python from warp (card ffi-python-adapter, notes/python_adapter.md): `use python` brings lib/native/python.warp, whose
# foreign lines warp-host links to libpython by dlopen and dlsym. Outcome checks: the printed values and exit statuses.
# Run it through the queue: tests/queue.sh tests/host/test_python.sh
# The host and compiler are found as tests/host/test_ffi.sh finds them; WARP_FFI_COMPILER=full tests warp.wasm.
# Without libpython (Linux libpython3.X.so.1.0, the package libpython3.X) every check is skipped; without numpy its one.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/python-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler=${WARP_FFI_COMPILER:-self}
compiler_file=$([ "$compiler" = full ] && echo warp.wasm || echo warp-self.wasm)
compiler_path=${WARP_FFI_COMPILER_FILE:-$(dirname "$rust")/$compiler_file}
[ -f "$compiler_path" ] || { echo "✗ no $compiler_file beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$compiler_path" "$stage/bin/$compiler_file" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
has_libpython() {
	local file
	for file in /usr/lib/*/libpython3.*.so* /lib/*/libpython3.*.so* /usr/lib64/libpython3.*.so* /usr/lib/libpython3.*.so* \
		/usr/local/lib/libpython3.*.so*; do
		[ -e "$file" ] && return 0
	done
	return 1
}

program sqrt 'use python' 'print python_call(python_import("math"), "sqrt", [16])'
program upper 'use python' 'print python_call("hello", "upper", [])'
program dumps_list 'use python' 'print python_call(python_import("json"), "dumps", [[1, "two", 3.5, yes, ø]])'
program dumps_map 'use python' 'print python_call(python_import("json"), "dumps", [{name: "Alice", age: 30}])'
program keywords 'use python' 'print python_call_with(python_import("json"), "dumps", [{b: 1, a: 2}], {sort_keys: yes})'
program attribute 'use python' 'print python_get(python_import("math"), "pi")'
program eval_list 'use python' 'print python_eval("sorted([3, 1, 2])")'
program eval_dict 'use python' 'print python_eval("{'"'"'k'"'"': (1, 2.5, None)}")'
program exec_def 'use python' 'python_exec("def square(x): return x * x")' 'print python_eval("square(12)")'
program raised 'use python' 'print python_call(python_import("math"), "sqrt", [-1])'
program caught 'use python' 'print try python_call(python_import("math"), "sqrt", [-1]) else "caught"'
program no_module 'use python' 'python_import("no_such_module")'
program reference 'use python' 'print python_text(python_call(python_import("fractions"), "Fraction", [3, 4]))'
program counter 'use python' 'c = python_call(python_import("collections"), "Counter", ["banana"])' 'print python_call(c, "most_common", [1])'
program numpy 'use python' 'np = python_import("numpy")' 'a = python_call(np, "array", [[[1, 2], [3, 4]]])' \
	'print python_call(python_call(a, "dot", [a]), "tolist", [])'

run=0 failed=0 skipped=0
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
runs() { "$host" --compiler "$compiler" run "$@"; }

if ! has_libpython; then
	echo "- every check skipped: no libpython3.X.so here (apt install libpython3.13)"
	exit 0
fi
expect "math.sqrt(16)" 0 "4" ! "Error" -- runs sqrt.warp
expect "a str method on a warp text" 0 "HELLO" -- runs upper.warp
expect "json.dumps of a warp list" 0 '[1, "two", 3.5, true, null]' -- runs dumps_list.warp
expect "json.dumps of a warp map" 0 '{"name": "Alice", "age": 30}' -- runs dumps_map.warp
expect "keyword arguments" 0 '{"a": 2, "b": 1}' -- runs keywords.warp
expect "an attribute: math.pi" 0 "3.141592653589793" -- runs attribute.warp
expect "a Python list back as a warp list" 0 "[1 2 3]" -- runs eval_list.warp
expect "a dict with a tuple and None back as a map" 0 "{k:[1 2.5 ø]}" -- runs eval_dict.warp
expect "exec defines, eval calls" 0 "144" -- runs exec_def.warp
expect "a Python exception is a warp Error" 1 "python ValueError: math domain error" -- runs raised.warp
expect "the Error is caught by try" 0 "caught" ! "ValueError" -- runs caught.warp
expect "a missing module" 1 "python ModuleNotFoundError: No module named 'no_such_module'" -- runs no_module.warp
expect "an object without a warp value stays a reference (fractions needs lib-dynload's _contextvars)" 0 "3/4" -- runs reference.warp
expect "methods of a reference" 0 "[['a' 3]]" -- runs counter.warp
if python3 -c 'import numpy' 2>/dev/null; then
	expect "numpy: an array's product with itself" 0 "[[7 10] [15 22]]" -- runs numpy.warp
else
	skipped=$((skipped + 1))
	echo "- numpy (not installed)"
fi

echo "python: $((run - failed)) passed / $run ($skipped skipped)"
[ "$failed" = 0 ]

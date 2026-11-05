#!/usr/bin/env bash
# `warp build x.warp` writes x.mod.wasm, the program alone importing runtime.wasm (decision build-three-outputs,
# notes/runtime_linking.md "Separate modules"): linked to an instance of the runtime.wasm it was built behind, it prints
# what the program prints. Node links the two (as web/playground/host.js does), WASI from node:wasi. Run it with
# bash test/native/test_separate_module.sh; the warp is WARP (default the warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
library=${WARP_LIB:-$("$warp" version | sed -n 's/.*(compiler.warp of \(.*\))$/\1/p')}
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

# linked <module>: the module run linked to an instance of runtime.wasm, its printed lines
linked() {
	node --no-warnings --input-type=module -e '
import { readFileSync } from "node:fs";
import { WASI } from "node:wasi";
const [runtimeFile, programFile] = process.argv.slice(1);
const wasi = new WASI({ version: "preview1", args: [programFile], env: {}, returnOnExit: true });
const instantiated = (file, given) => {
	const module = new WebAssembly.Module(readFileSync(file));
	const imports = {};
	for (const { module: from, name, kind } of WebAssembly.Module.imports(module)) {
		imports[from] ??= {};
		imports[from][name] = given[from]?.[name] ?? (kind === "function" ? () => { throw new Error(`no host word ${from}.${name}`); } : undefined);
	}
	return new WebAssembly.Instance(module, imports);
};
const runtime = instantiated(runtimeFile, { wasi_snapshot_preview1: wasi.wasiImport });
wasi.start(instantiated(programFile, { warp_runtime: runtime.exports, wasi_snapshot_preview1: wasi.wasiImport }));
' "$library/runtime.wasm" "$1" 2>&1
}
# check <name> <program> <expected>: built by `warp build`, its x.mod.wasm linked prints the expected
check() {
	printf '%s\n' "$2" >"$stage/$1.warp"
	(cd "$stage" && "$warp" build --wasm "$1.warp" >"$stage/$1.log" 2>&1)
	got=$(linked "$stage/$1.mod.wasm")
	if [ "$got" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $got, expected $3 ($(tail -2 "$stage/$1.log"))"; failures=$((failures + 1)); fi
}

check arithmetic 'print "Hi " + (6*7)' "Hi 42"
check class_text 'class P{x:int; text() := "P" + x}
print P(3)' "P3"
check function 'square(n: int): int := n*n
print square(12)' "144"

# natively: the warp runs x.mod.wasm itself, linking runtime.wasm's instance as `warp_runtime` (card
# wasm-module-linking, core/native/driver.warp instances_linked)
native_check() {
	printf '%s\n' "$2" >"$stage/$1.warp"
	(cd "$stage" && "$warp" build --wasm "$1.warp" >"$stage/$1.log" 2>&1)
	got=$("$warp" "$stage/$1.mod.wasm" 2>&1)
	if [ "$got" = "$3" ]; then echo "✓ native $1"; else echo "✗ native $1: got $got, expected $3"; failures=$((failures + 1)); fi
}
native_check arithmetic 'print "Hi " + (6*7)' "Hi 42"
native_check class_text 'class P{x:int; text() := "P" + x}
print P(3)' "P3"
[ $failures = 0 ]

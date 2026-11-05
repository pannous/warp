#!/usr/bin/env bash
# Every standard module `warp rebuild` writes as modules/<name>.wasm links at load time (card wasm-module-linking,
# notes/library_zero_bake.md "The module interface"): a program importing from `warp/<name>` runs, the module linked to
# runtime.wasm's instance and its warp/start run first. Needs wat2wasm (wabt). Run it with
# bash test/native/test_linked_modules.sh; the warp is WARP (default the warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
library=${WARP_LIB:-$("$warp" version | sed -n 's/.*(compiler.warp of \(.*\))$/\1/p')}
command -v wat2wasm >/dev/null || { echo "skipped: no wat2wasm"; exit 0; }
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
for file in "$library"/modules/*.wasm; do
	case $file in *.splice.wasm) continue ;; esac
	[ -s "$file" ] || continue
	name=$(basename "$file" .wasm)
	printf '(module (import "warp/%s" "memory" (memory 0)) (func (export "main") (result i64) (i64.const 7)))' "$name" >"$stage/$name.wat"
	wat2wasm "$stage/$name.wat" -o "$stage/$name.wasm"
	got=$("$warp" "$stage/$name.wasm" 2>&1 | tail -1)
	if [ "$got" = 7 ]; then echo "✓ $name"; else echo "✗ $name: $got"; failures=$((failures + 1)); fi
done
[ $failures = 0 ]

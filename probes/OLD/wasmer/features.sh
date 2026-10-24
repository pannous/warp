#!/bin/sh
# One tiny module per wasm proposal warp uses, run under each wasmer backend and under wasmtime (notes/wasmer.md).
# WASMER: the wasmer binary; the official release tarball has --cranelift --llvm --singlepass --v8, brew's only Cranelift.
WASMER=${WASMER:-wasmer}
BACKENDS=${BACKENDS:---cranelift --llvm --singlepass --v8}
OUT=$(cd "$(dirname "$0")/../.." && pwd)/scratch/wasmer-probes
mkdir -p "$OUT" && cd "$OUT" || exit 1

first_line() { grep -v -e warning -e '^\s*$' | head -1 | cut -c1-70; }

probe() { # name wat
  printf '%s\n' "$2" > "$1.wat"
  wasm-tools parse "$1.wat" -o "$1.wasm" || return
  printf '%-13s wasmtime: %s\n' "$1" "$(wasmtime run -W gc=y,function-references=y,exceptions=y,tail-call=y,multi-memory=y --invoke main "$1.wasm" 2>&1 | first_line)"
  for backend in $BACKENDS; do
    printf '%-13s %-13s %s\n' '' "$backend" "$("$WASMER" run "$backend" --invoke main "$1.wasm" 2>&1 | first_line)"
  done
}

probe mvp '(module (func (export "main") (result i32) i32.const 42))'
probe tail_call '(module (func $f (result i32) i32.const 42) (func (export "main") (result i32) return_call $f))'
probe multi_memory '(module (memory 1) (memory $b 1) (func (export "main") (result i32) i32.const 0 i32.const 42 i32.store $b i32.const 0 i32.load $b))'
probe exceptions '(module (tag $t (param i32)) (func (export "main") (result i32) (block $h (result i32) (try_table (catch $t $h) i32.const 42 throw $t) unreachable)))'
probe func_refs '(module (type $ft (func (result i32))) (func $f (type $ft) i32.const 42) (elem declare func $f) (func (export "main") (result i32) ref.func $f call_ref $ft))'
probe i31 '(module (func (export "main") (result i32) i32.const 42 ref.i31 i31.get_s))'
probe gc_struct '(module (type $s (struct (field i32))) (func (export "main") (result i32) i32.const 42 struct.new $s struct.get $s 0))'
probe gc_array '(module (type $a (array (mut i8))) (func (export "main") (result i32) i32.const 42 i32.const 3 array.new $a i32.const 1 array.get_u $a))'

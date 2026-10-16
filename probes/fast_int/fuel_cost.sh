#!/bin/sh
# cards fast-int, fuel-metering: what a warp loop costs against plain wasm, and what fuel costs a call.
# Builds the WAT below into scratch/, compiles exact_loop.warp and fast_loop.warp, times them with hyperfine.
# Exit codes are results mod 256, hence -i.
set -e
here=$(cd "$(dirname "$0")" && pwd)
out="$here/../../scratch/fast_int"
mkdir -p "$out"
cat > "$out/raw_loop.wat" <<'WAT'
(module
  (func $loop (param $n i64) (result i64) (local $i i64) (local $total i64)
    (block (loop
      (br_if 1 (i64.ge_s (local.get $i) (local.get $n)))
      (local.set $total (i64.add (local.get $total) (i64.rem_s (local.get $i) (i64.const 7))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br 0)))
    (local.get $total))
  (func (export "main") (result i64) (call $loop (i64.const 100000000))))
WAT
cat > "$out/ref_calls.wat" <<'WAT'
(module
  (type $S (struct (field i32)))
  (func $take_ref (param (ref $S)) (param i64) (result i64) (i64.add (local.get 1) (i64.extend_i32_u (struct.get $S 0 (local.get 0)))))
  (func $take_int (param i32) (param i64) (result i64) (i64.add (local.get 1) (i64.extend_i32_u (local.get 0))))
  (func $refs (export "refs") (result i64) (local $s (ref null $S)) (local $i i64) (local $t i64)
    (local.set $s (struct.new $S (i32.const 1)))
    (block (loop (br_if 1 (i64.ge_s (local.get $i) (i64.const 40000000)))
      (local.set $t (call $take_ref (ref.as_non_null (local.get $s)) (local.get $t)))
      (local.set $i (i64.add (local.get $i) (i64.const 1))) (br 0)))
    (local.get $t))
  (func $ints (export "ints") (result i64) (local $i i64) (local $t i64)
    (block (loop (br_if 1 (i64.ge_s (local.get $i) (i64.const 40000000)))
      (local.set $t (call $take_int (i32.const 1) (local.get $t)))
      (local.set $i (i64.add (local.get $i) (i64.const 1))) (br 0)))
    (local.get $t)))
WAT
wasm-tools parse "$out/raw_loop.wat" -o "$out/raw_loop.wasm"
for variant in refs ints; do
	sed "s/(export \"$variant\")/(export \"main\")/" "$out/ref_calls.wat" | wasm-tools parse - -o "$out/$variant.wasm"
done
warp="${WARP:-$here/../../scratch/warp}"
for program in exact_loop fast_loop; do
	WARP_NO_WINDOW=1 WARP_HINTS=0 "$warp" compile --wasm "$here/$program.warp" >/dev/null
	mv "$here/$program.wasm" "$out/"
done
cd "$out"
hyperfine -N -i -w 1 -r 5 'warp raw_loop.wasm' 'warp exact_loop.wasm' 'warp fast_loop.wasm'
gc='-W gc=y,function-references=y --invoke main'
hyperfine -N -i -w 1 -r 5 "wasmtime run $gc refs.wasm" "wasmtime run $gc -W fuel=100000000000 refs.wasm" 'warp refs.wasm'

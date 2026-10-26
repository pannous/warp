;; built into greeter.wasm (found first, what the browser build reads) by `wasm-tools parse tests/fixtures/wasm/greeter.wat -o tests/fixtures/wasm/greeter.wasm`
;; an imported module that imports itself: WASI (fd_write), a warp host word (random_below) and another module
;; (counter.wasm), linked with the run's import families (tests/modules/test_wasm_modules.rs)
(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "host" "random_below" (func $random_below (param i64) (result i64)))
  (import "tests/fixtures/wasm/counter.wasm" "count_up" (func $count_up (param i64) (result i64)))
  (memory (export "memory") 1)
  (data (i32.const 16) "hi\n")
  ;; prints hi, the number of bytes written
  (func (export "greet") (result i32)
    (i32.store (i32.const 0) (i32.const 16))
    (i32.store (i32.const 4) (i32.const 3))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))
    (i32.load (i32.const 8)))
  ;; random_below(1) is always 0
  (func (export "five") (result i64)
    (i64.add (call $random_below (i64.const 1)) (i64.const 5)))
  (func (export "count_twice") (param $by i64) (result i64)
    (drop (call $count_up (local.get $by)))
    (call $count_up (local.get $by))))

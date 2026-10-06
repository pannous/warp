;; built into counter.wasm by `wasm-tools parse tests/fixtures/wasm/counter.wat -o tests/fixtures/wasm/counter.wasm`:
;; the binary form of an imported module (tests/modules/test_wasm_modules.rs)
(module
  (global $count (mut i64) (i64.const 0))
  (func (export "count_up") (param $by i64) (result i64)
    (global.set $count (i64.add (global.get $count) (local.get $by)))
    (global.get $count)))

(module $wasp_compact
  (type (;0;) (func (param f64 f64) (result f64)))
  (type $String (;1;) (struct (field $ptr i32) (field $len i32)))
  (type $Node (;2;) (struct (field $kind i64) (field $data (mut anyref)) (field $value (ref null $Node))))
  (type $i64box (;3;) (struct (field $value i64)))
  (type $f64box (;4;) (struct (field $value f64)))
  (type (;5;) (func (param f64) (result (ref $Node))))
  (type (;6;) (func (param i32) (result (ref $Node))))
  (type (;7;) (func (param i32 i32) (result (ref $Node))))
  (type (;8;) (func (param (ref null $Node) (ref null $Node) i64) (result (ref $Node))))
  (type (;9;) (func (param (ref null $Node)) (result i64)))
  (type (;10;) (func (result (ref $Node))))
  (import "libm" "fmin" (func $ffi_fmin (;0;) (type 0)))
  (memory (;0;) 1)
  (global $kind_empty (;0;) i64 i64.const 0)
  (global $kind_int (;1;) i64 i64.const 1)
  (global $kind_float (;2;) i64 i64.const 2)
  (global $kind_text (;3;) i64 i64.const 3)
  (global $kind_codepoint (;4;) i64 i64.const 4)
  (global $kind_symbol (;5;) i64 i64.const 5)
  (global $kind_key (;6;) i64 i64.const 6)
  (global $kind_block (;7;) i64 i64.const 7)
  (global $kind_list (;8;) i64 i64.const 8)
  (global $kind_data (;9;) i64 i64.const 9)
  (global $kind_meta (;10;) i64 i64.const 10)
  (global $kind_error (;11;) i64 i64.const 11)
  (global (;12;) i64 i64.const 12)
  (export "memory" (memory 0))
  (export "kind_empty" (global $kind_empty))
  (export "kind_int" (global $kind_int))
  (export "kind_float" (global $kind_float))
  (export "kind_text" (global $kind_text))
  (export "kind_codepoint" (global $kind_codepoint))
  (export "kind_symbol" (global $kind_symbol))
  (export "kind_key" (global $kind_key))
  (export "kind_block" (global $kind_block))
  (export "kind_list" (global $kind_list))
  (export "kind_data" (global $kind_data))
  (export "kind_meta" (global $kind_meta))
  (export "kind_error" (global $kind_error))
  (export "kind_type" (global 12))
  (export "new_float" (func $new_float))
  (export "new_codepoint" (func $new_codepoint))
  (export "new_symbol" (func $new_symbol))
  (export "new_list" (func $new_list))
  (export "get_kind" (func $get_kind))
  (export "main" (func 6))
  (func $new_float (;1;) (type 5) (param f64) (result (ref $Node))
    global.get $kind_float
    local.get 0
    struct.new $f64box
    ref.null $Node
    struct.new $Node
  )
  (func $new_codepoint (;2;) (type 6) (param i32) (result (ref $Node))
    global.get $kind_codepoint
    local.get 0
    ref.i31
    ref.null $Node
    struct.new $Node
  )
  (func $new_symbol (;3;) (type 7) (param i32 i32) (result (ref $Node))
    global.get $kind_symbol
    local.get 0
    local.get 1
    struct.new $String
    ref.null $Node
    struct.new $Node
  )
  (func $new_list (;4;) (type 8) (param (ref null $Node) (ref null $Node) i64) (result (ref $Node))
    local.get 2
    i64.const 8
    i64.shl
    global.get $kind_list
    i64.or
    local.get 0
    local.get 1
    struct.new $Node
  )
  (func $get_kind (;5;) (type 9) (param (ref null $Node)) (result i64)
    local.get 0
    struct.get $Node $kind
  )
  (func (;6;) (type 10) (result (ref $Node))
    f64.const 0x1.cp+1 (;=3.5;)
    f64.const 0x1.0cccccccccccdp+1 (;=2.1;)
    call $ffi_fmin
    call $new_float
  )
  (data (;0;) (i32.const 8) "import")
  (data (;1;) (i32.const 14) "fmin")
  (data (;2;) (i32.const 18) "from")
)

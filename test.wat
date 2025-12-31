(module $wasp_node_ast
  (type $node (;0;) (struct (field $name_ptr i32) (field $name_len i32) (field $tag i32) (field $int_value i64) (field $float_value f64) (field $text_ptr i32) (field $text_len i32) (field $left (ref null $node)) (field $right (ref null $node)) (field $meta (ref null $node))))
  (type $node_array (;1;) (array (mut (ref null $node))))
  (type (;2;) (func (result (ref $node))))
  (type (;3;) (func (param i64) (result (ref $node))))
  (type (;4;) (func (param f64) (result (ref $node))))
  (type (;5;) (func (param i32) (result (ref $node))))
  (type (;6;) (func (param i32 i32) (result (ref $node))))
  (type (;7;) (func (param i32 i32) (result (ref $node))))
  (type (;8;) (func (param i32 i32 (ref null $node) (ref null $node)) (result (ref $node))))
  (type (;9;) (func (param (ref null $node) (ref null $node)) (result (ref $node))))
  (type (;10;) (func (param i32 i32 (ref null $node)) (result (ref $node))))
  (type (;11;) (func (param (ref null $node)) (result i32)))
  (type (;12;) (func (param (ref $node)) (result i32)))
  (type (;13;) (func (param (ref $node)) (result i64)))
  (type (;14;) (func (param (ref $node)) (result f64)))
  (type (;15;) (func (param (ref $node)) (result i32)))
  (type (;16;) (func (result (ref $node))))
  (memory (;0;) 1)
  (export "memory" (memory 0))
  (export "new_empty" (func $new_empty))
  (export "new_int" (func $new_int))
  (export "new_float" (func $new_float))
  (export "new_codepoint" (func $new_codepoint))
  (export "new_text" (func $new_text))
  (export "new_symbol" (func $new_symbol))
  (export "new_tag" (func $new_tag))
  (export "new_pair" (func $new_pair))
  (export "new_keyvalue" (func $new_keyvalue))
  (export "get_node_kind" (func $get_node_kind))
  (export "get_tag" (func $get_tag))
  (export "get_int_value" (func $get_int_value))
  (export "get_float_value" (func $get_float_value))
  (export "get_name_len" (func $get_name_len))
  (export "main" (func $main))
  (func $new_empty (;0;) (type 2) (result (ref $node))
    i32.const 0
    i32.const 0
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_int (;1;) (type 3) (param i64) (result (ref $node))
    (local i64)
    i32.const 0
    i32.const 0
    i32.const 1
    local.get 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_float (;2;) (type 4) (param f64) (result (ref $node))
    (local f64)
    i32.const 0
    i32.const 0
    i32.const 1
    i64.const 0
    local.get 0
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_codepoint (;3;) (type 5) (param i32) (result (ref $node))
    (local i32)
    i32.const 0
    i32.const 0
    i32.const 3
    local.get 0
    i64.extend_i32_u
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_text (;4;) (type 6) (param i32 i32) (result (ref $node))
    (local i32 i32)
    i32.const 0
    i32.const 0
    i32.const 2
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get 0
    local.get 1
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_symbol (;5;) (type 7) (param i32 i32) (result (ref $node))
    (local i32 i32)
    i32.const 0
    i32.const 0
    i32.const 4
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get 0
    local.get 1
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_tag (;6;) (type 8) (param i32 i32 (ref null $node) (ref null $node)) (result (ref $node))
    (local i32 i32 (ref null $node) (ref null $node))
    local.get 0
    local.get 1
    i32.const 7
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    local.get 2
    local.get 3
    ref.null $node
    struct.new $node
  )
  (func $new_pair (;7;) (type 9) (param (ref null $node) (ref null $node)) (result (ref $node))
    (local (ref null $node) (ref null $node))
    i32.const 0
    i32.const 0
    i32.const 6
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    local.get 0
    local.get 1
    ref.null $node
    struct.new $node
  )
  (func $new_keyvalue (;8;) (type 10) (param i32 i32 (ref null $node)) (result (ref $node))
    (local i32 i32 (ref null $node))
    local.get 0
    local.get 1
    i32.const 5
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    local.get 2
    ref.null $node
    struct.new $node
  )
  (func $get_node_kind (;9;) (type 11) (param (ref null $node)) (result i32)
    (local (ref null $node))
    local.get 0
    struct.get $node $tag
  )
  (func $get_tag (;10;) (type 12) (param (ref $node)) (result i32)
    (local (ref $node))
    local.get 0
    struct.get $node $tag
  )
  (func $get_int_value (;11;) (type 13) (param (ref $node)) (result i64)
    (local (ref $node))
    local.get 0
    struct.get $node $int_value
  )
  (func $get_float_value (;12;) (type 14) (param (ref $node)) (result f64)
    (local (ref $node))
    local.get 0
    struct.get $node $float_value
  )
  (func $get_name_len (;13;) (type 15) (param (ref $node)) (result i32)
    (local (ref $node))
    local.get 0
    struct.get $node $name_len
  )
  (func $main (;14;) (type 16) (result (ref $node))
    i32.const 8
    i32.const 4
    call $new_empty
    i32.const 12
    i32.const 4
    i64.const 1
    call $new_int
    call $new_keyvalue
    call $new_tag
  )
  (data (;0;) (i32.const 8) "html")
  (data (;1;) (i32.const 12) "test")
)

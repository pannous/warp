(module $wasp_node_ast
  (type $node (;0;) (struct (field $name_ptr i32) (field $name_len i32) (field $tag i32) (field $int_value i64) (field $float_value f64) (field $text_ptr i32) (field $text_len i32) (field $left (ref null $node)) (field $right (ref null $node)) (field $meta (ref null $node))))
  (type $node_array (;1;) (array (mut (ref null $node))))
  (type (;2;) (func (result (ref $node))))
  (type (;3;) (func (param i64) (result (ref $node))))
  (type (;4;) (func (param f64) (result (ref $node))))
  (type (;5;) (func (param i32) (result (ref $node))))
  (type (;6;) (func (param (ref null $node)) (result i32)))
  (type (;7;) (func (param (ref $node)) (result i32)))
  (type (;8;) (func (param (ref $node)) (result i64)))
  (type (;9;) (func (param (ref $node)) (result f64)))
  (type (;10;) (func (param (ref $node)) (result i32)))
  (type (;11;) (func (result (ref $node))))
  (export "make_empty" (func $make_empty))
  (export "make_int" (func $make_int))
  (export "make_float" (func $make_float))
  (export "make_codepoint" (func $make_codepoint))
  (export "get_node_kind" (func $get_node_kind))
  (export "get_tag" (func $get_tag))
  (export "get_int_value" (func $get_int_value))
  (export "get_float_value" (func $get_float_value))
  (export "get_name_len" (func $get_name_len))
  (export "main" (func $main))
  (func $make_empty (;0;) (type 2) (result (ref $node))
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
  (func $make_int (;1;) (type 3) (param i64) (result (ref $node))
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
  (func $make_float (;2;) (type 4) (param f64) (result (ref $node))
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
  (func $make_codepoint (;3;) (type 5) (param i32) (result (ref $node))
    (local i32)
    i32.const 0
    i32.const 0
    i32.const 3
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $get_node_kind (;4;) (type 6) (param (ref null $node)) (result i32)
    (local (ref null $node))
    local.get 0
    struct.get $node $tag
  )
  (func $get_tag (;5;) (type 7) (param (ref $node)) (result i32)
    (local (ref $node))
    local.get 0
    struct.get $node $tag
  )
  (func $get_int_value (;6;) (type 8) (param (ref $node)) (result i64)
    (local (ref $node))
    local.get 0
    struct.get $node $int_value
  )
  (func $get_float_value (;7;) (type 9) (param (ref $node)) (result f64)
    (local (ref $node))
    local.get 0
    struct.get $node $float_value
  )
  (func $get_name_len (;8;) (type 10) (param (ref $node)) (result i32)
    (local (ref $node))
    local.get 0
    struct.get $node $name_len
  )
  (func $main (;9;) (type 11) (result (ref $node))
    i32.const 0
    i32.const 4
    i32.const 7
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
)

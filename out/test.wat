(module $wasp_node_ast
  (type $node (;0;) (struct
  (field $name_ptr i32) (field $name_len i32) (field $tag i32) (field $int_value i64) (field $float_value f64)
  (field $text_ptr i32) (field $text_len i32) (field $left (ref null $node)) (field $right (ref null $node))
  (field $meta (ref null $node))))
  (type $node_array (;1;) (array (mut (ref null $node))))
  (type $#type2 (;2;) (func (result (ref $node))))
  (type $#type3 (;3;) (func (param i64) (result (ref $node))))
  (type $#type4 (;4;) (func (param f64) (result (ref $node))))
  (type $#type5 (;5;) (func (param i32) (result (ref $node))))
  (type $#type6 (;6;) (func (param i32 i32) (result (ref $node))))
  (type $#type7 (;7;) (func (param i32 i32) (result (ref $node))))
  (type $#type8 (;8;) (func (param i32 i32 (ref null $node) (ref null $node)) (result (ref $node))))
  (type $#type9 (;9;) (func (param (ref null $node) (ref null $node)) (result (ref $node))))
  (type $#type10 (;10;) (func (param i32 i32 (ref null $node)) (result (ref $node))))
  (type $#type11 (;11;) (func (param (ref null $node)) (result i32)))
  (type $#type12 (;12;) (func (param (ref $node)) (result i32)))
  (type $#type13 (;13;) (func (param (ref $node)) (result i64)))
  (type $#type14 (;14;) (func (param (ref $node)) (result f64)))
  (type $#type15 (;15;) (func (param (ref $node)) (result i32)))
  (type $#type16 (;16;) (func (result (ref $node))))
  (memory $#memory0 (;0;) 1)
  (export "memory" (memory $#memory0))
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
  (func $new_empty (;0;) (type $#type2) (result (ref $node))
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
  (func $new_int (;1;) (type $#type3) (param $#local0 i64) (result (ref $node))
    (local $#local1 i64)
    i32.const 0
    i32.const 0
    i32.const 1
    local.get $#local0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_float (;2;) (type $#type4) (param $#local0 f64) (result (ref $node))
    (local $#local1 f64)
    i32.const 0
    i32.const 0
    i32.const 1
    i64.const 0
    local.get $#local0
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_codepoint (;3;) (type $#type5) (param $#local0 i32) (result (ref $node))
    (local $#local1 i32)
    i32.const 0
    i32.const 0
    i32.const 3
    local.get $0
    i64.extend_i32u
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_text (;4;) (type $#type6) (param $#local0 i32) (param $#local1 i32) (result (ref $node))
    (local $#local2 i32) (local $#local3 i32)
    i32.const 0
    i32.const 0
    i32.const 2
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get $#local0
    local.get $#local1
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_symbol (;5;) (type $#type7) (param $#local0 i32) (param $#local1 i32) (result (ref $node))
    (local $#local2 i32) (local $#local3 i32)
    i32.const 0
    i32.const 0
    i32.const 4
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get $#local0
    local.get $#local1
    ref.null $node
    ref.null $node
    ref.null $node
    struct.new $node
  )
  (func $new_tag (;6;) (type $#type8) (param $#local0 i32) (param $#local1 i32) (param $#local2 (ref null $node)) (param $#local3 (ref null $node)) (result (ref $node))
    (local $#local4 i32) (local $#local5 i32) (local $#local6 (ref null $node)) (local $#local7 (ref null $node))
    local.get $#local0
    local.get $#local1
    i32.const 7
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    local.get $#local2
    local.get $#local3
    ref.null $node
    struct.new $node
  )
  (func $new_pair (;7;) (type $#type9) (param $#local0 (ref null $node)) (param $#local1 (ref null $node)) (result (ref $node))
    (local $#local2 (ref null $node)) (local $#local3 (ref null $node))
    i32.const 0
    i32.const 0
    i32.const 6
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    local.get $#local0
    local.get $#local1
    ref.null $node
    struct.new $node
  )
  (func $new_keyvalue (;8;) (type $#type10) (param $#local0 i32) (param $#local1 i32) (param $#local2 (ref null $node)) (result (ref $node))
    (local $#local3 i32) (local $#local4 i32) (local $#local5 (ref null $node))
    local.get $#local0
    local.get $#local1
    i32.const 5
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    i32.const 0
    i32.const 0
    ref.null $node
    local.get $#local2
    ref.null $node
    struct.new $node
  )
  (func $get_node_kind (;9;) (type $#type11) (param $#local0 (ref null $node)) (result i32)
    (local $#local1 (ref null $node))
    local.get $#local0
    struct.get $node $tag
  )
  (func $get_tag (;10;) (type $#type12) (param $#local0 (ref $node)) (result i32)
    (local $#local1 (ref $node))
    local.get $#local0
    struct.get $node $tag
  )
  (func $get_int_value (;11;) (type $#type13) (param $#local0 (ref $node)) (result i64)
    (local $#local1 (ref $node))
    local.get $#local0
    struct.get $node $int_value
  )
  (func $get_float_value (;12;) (type $#type14) (param $#local0 (ref $node)) (result f64)
    (local $#local1 (ref $node))
    local.get $#local0
    struct.get $node $float_value
  )
  (func $get_name_len (;13;) (type $#type15) (param $#local0 (ref $node)) (result i32)
    (local $#local1 (ref $node))
    local.get $#local0
    struct.get $node $name_len
  )
  (func $main (;14;) (type $#type16) (result (ref $node))
    i32.const 8
    i32.const 3
    i64.const 123
    call $new_int
    call $new_keyvalue
  )
  (data $#data0 (;0;) (i32.const 8) "key")
)

(module $wasp_node_ast
;; demonstration of the tree node structure in WebAssembly Text Format
  (type $node (;0;) (struct
    (field $name_ptr i32)
    (field $name_len i32)
    (field $tag i32)
    (field $int_value i64)
    (field $float_value f64)
    (field $text_ptr i32)
    (field $text_len i32)
    (field $left (ref null $node))
    (field $right (ref null $node))
    (field $meta (ref null $node))))
  (type $node_array (;1;) (array (mut (ref null $node))))
)

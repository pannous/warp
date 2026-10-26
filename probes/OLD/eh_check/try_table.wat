(module
  (tag $e (param i32))
  (func (export "main") (result i32)
    block (result i32)
      try_table (catch $e 0)
        i32.const 7
        throw $e
      end
      i32.const 0
    end))

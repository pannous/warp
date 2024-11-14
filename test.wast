(module
  (type (;0;) (func (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (func (;0;) (type 0) (result i32)
    i32.const 42)
  (func (;1;) (type 1) (param i32 i32) (result i32)
    (local i32)
    i32.const 42
    i32.const 4
    i32.mul)
  (export "main" (func 0)))

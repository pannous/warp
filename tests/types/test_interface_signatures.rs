//! An interface member typed as a function, as WIT and TypeScript write it (`add: (i32, i32) -> i32`), is an operation
//! of that many parameters (samples/wasm_interop.wasp; it was 'trait calculator takes operations like `area`')
use crate::is;

#[test]
fn a_function_typed_member_is_an_operation() {
	is!("interface calculator { add: (i32, i32) -> i32; sub: (i32, i32) -> i32 }; 1", 1);
	is!("interface calculator { add: (i32, i32) -> i32 }; add(a: int, b: int) := a + b; add(2, 3)", 5);
	is!("interface shape { area: (int) -> float }; class sq { s: int }; area(x: sq) := x.s * x.s; area(sq{s: 3})", 9);
}

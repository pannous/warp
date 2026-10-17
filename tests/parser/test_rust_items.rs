//! card rust-items: Rust items end at their closing brace, so `struct P { … } let p = …` on one line is two statements,
//! as after a newline (decision rust-forms-welcome); the warp form is `class P { x:int }` with a newline or `;`
use crate::is;

#[test]
fn an_item_ends_at_its_closing_brace() {
	is!("struct P { x: i32, y: i32 } let p = P { x: 3, y: 4 }; p.x", 3);
	is!("class P { x: int } let p = P { x: 5 }; p.x", 5);
	is!("enum C { Red, Green } let c = C::Green; match c { C::Red => 1, C::Green => 2 }", 2);
	is!("struct P { x: i32 } impl P { fn double(&self) -> i32 { self.x * 2 } } let p = P { x: 3 }; p.double()", 6);
	is!("fn three() -> i32 { 3 } let x = three(); x + 1", 4);
}

#[test]
fn impl_trait_for_a_struct_gives_it_the_methods() {
	is!("struct S { a: i32 }\nimpl Area for S { fn area(&self) -> i32 { self.a * self.a } }\nS { a: 2 }.area()", 4);
	is!(
		"trait Area { fn area(&self) -> i32; } struct S { a: i32 } impl Area for S { fn area(&self) -> i32 { self.a * self.a } } S { a: 2 }.area()",
		4
	);
}

#[test]
fn a_block_value_still_continues() {
	is!("if 1 > 0 { 2 } else { 3 }", 2);
	is!("x = 1; if x > 0 { x = 7 }; x", 7);
}

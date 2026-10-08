//! A class field of the class's own type (`left: Tree?`) refers to the class: trees and linked lists as GC structs;
//! WebAssembly's spellings `type Node: gc struct {…}` and `left: ref Node?` mean the same (samples/wasm_interop.warp)
use crate::is;

#[test]
fn a_field_of_the_class_itself_holds_an_instance() {
	is!("class T { v: int; left: T?; right: T? }; t = T{v: 1 left: T{v: 2 left: ø right: ø} right: ø}; t.left.v + t.v", 3);
	is!("class Person { name: text; friend: Person? }; b = Person{name:\"bo\" friend: ø}; a = Person{name:\"al\" friend: b}; a.friend.name", "bo");
}

#[test]
fn gc_struct_and_ref_are_wasm_spellings() {
	is!("type N: gc struct { tag: i32 value: i64 }; n = N { tag: 1 value: 42 }; n.value", 42);
	is!("type N: gc struct { tag: i32 left: ref N? }; n = N { tag: 7 left: ø }; n.tag", 7);
}

#[test]
fn ref_fields_on_their_own_lines() {
	is!("type Node: gc struct {\n    tag: i32\n    left: ref Node?\n    right: ref Node?\n}\ntree = Node {\n    tag: 5\n    left: ø\n    right: ø\n}\ntree.tag", 5);
}

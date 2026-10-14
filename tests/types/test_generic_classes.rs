// Generic classes `class Box<T>{item:T}` (Java, Kotlin, Swift, TypeScript, Rust): a field or parameter of a type
// parameter holds any value
use crate::is;

#[test]
fn a_generic_class_holds_any_value() {
	is!("class Box<T>{item:T}; Box(3).item", 3);
	is!("class Box<T>{item:T; get() := item}; b = Box(\"hi\"); b.get()", "hi");
	is!("class Pair<A, B>{first:A; second:B}; p = Pair(1, \"one\"); p.second", "one");
	is!("class Box<T>{item:T; with(x:T) := Box(x)}; Box(1).with(2).item", 2);
}

#[test]
fn type_arguments_of_a_generic_class_construct_it() {
	is!("class Box<T>{item:T}; b = Box<int>(3); b.item", 3);
	is!("class Box<T>{item:T}; Box<int>{item:3}.item", 3);
	is!("class Box<T>{item:T}; b:Box<int> = Box(3); b.item", 3);
}

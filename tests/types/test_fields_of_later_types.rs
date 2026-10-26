//! card node-enum-order: a field may name a type declared further down the file, as enum Node's `op: Op` in
//! core/node.warp names enum Op declared after it
use crate::is;

#[test]
fn a_field_of_a_class_declared_later() {
	is!("class A { b: B }\nclass B { n: int }\nA{b: B{n: 3}}.b.n", 3);
	is!("class A { c: C }\nclass C { b: B }\nclass B { n: int }\nA{c: C{b: B{n: 4}}}.c.b.n", 4);
}

#[test]
fn a_case_field_of_an_enum_declared_later() {
	is!(
		"enum Shape { Circle(r: Size) }\nenum Size { Small, Big }\nmatch Circle(Size.Big) { Circle(r): r as text }",
		"Big"
	);
}

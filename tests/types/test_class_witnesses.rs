// A class's own text, equality and order as methods (notes/traits.md: methods in the class body as sugar for the
// witnesses text·T, equals·T, compare·T), under their warp names and as other languages spell them
use crate::is;

#[test]
fn a_text_method_is_how_an_instance_prints() {
	is!("class P{x:int; text() := \"P\" + x}; P(3) as text", "P3");
	is!("class P{x:int; toString() := \"P\" + x}; P(3) as text", "P3");
	is!("class P{x:int; to_s() := \"P\" + x}; P(3) as text", "P3");
	is!("class P:\n    def __init__(self, x):\n        self.x = x\n    def __str__(self):\n        return \"P\" + str(self.x)\nP(3) as text", "P3");
	is!("class P{x:int; text() := \"P\" + x}; p = P(3); \"is \\(p)\"", "is P3");
}

#[test]
fn an_equals_method_is_how_instances_compare() {
	let p = "class P{x:int; y:int; equals(o) := x == o.x}; ";
	is!(&format!("{p}P(3, 1) == P(3, 2)"), true);
	is!(&format!("{p}P(3, 1) == P(4, 1)"), false);
	is!("class P{x:int; y:int; Equals(o) := x == o.x}; P(3, 1) == P(3, 2)", true);
}

#[test]
fn a_compare_method_is_how_instances_order() {
	is!("class P{x:int; compare(o) := x - o.x}; P(3) < P(4)", true);
	is!("class P{x:int; compareTo(o) := x - o.x}; P(5) < P(4)", false);
	is!("class P{x:int; compareTo(o) := x - o.x}; (sort [P(3), P(1), P(2)])#1.x", 1);
}

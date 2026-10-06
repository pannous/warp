// Operators on instances (wiki/operator.md: `a + b` invokes the object's `plus`/`add`): a class defines the method
// of an operator by its english name, or as Python (`__add__`) and Kotlin (`operator fun plus`) write it
use crate::is;

#[test]
fn a_class_defines_an_operator_by_its_name() {
	let vector = "class V{x:int; y:int; plus(o) := V(x + o.x, y + o.y); times(k) := V(x * k, y * k)}; ";
	is!(&format!("{vector}(V(1, 2) + V(3, 4)).y"), 6);
	is!(&format!("{vector}a = V(1, 2); b = V(3, 4); c = a + b; c.x"), 4);
	is!(&format!("{vector}a = V(1, 2); (a * 3).y"), 6);
	is!(&format!("{vector}a = V(1, 1); (a + a + a).x"), 3);
}

#[test]
fn operators_as_python_and_kotlin_write_them() {
	is!("class V:\n    def __init__(self, x):\n        self.x = x\n    def __add__(self, other):\n        return V(self.x + other.x)\n(V(1) + V(2)).x", 3);
	is!("data class V(val x: Int) { operator fun plus(o: V) = V(x + o.x) }\n(V(1) + V(2)).x", 3);
}

#[test]
fn a_class_defines_an_operator_by_its_glyph() {
	is!("class V{x:int; +(o) := V(x + o.x)}; (V(1) + V(2)).x", 3);
	is!("class V{x:int; -(o) := V(x - o.x)}; (V(5) - V(2)).x", 3);
	is!("class V{x:int; *(k) := V(x * k)}; (V(2) * 3).x", 6);
	is!("class V{x:int; operator +(o) := V(x + o.x)}; (V(1) + V(2)).x", 3);
}

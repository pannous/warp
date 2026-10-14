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

#[test]
fn an_operator_on_a_value_of_unknown_class_dispatches_at_run_time() {
	let classes = "class V{x:int; plus(o) := V(x + o.x)}; class W{y:int; plus(o) := W(y - o.y)}; ";
	is!(&format!("{classes}t = 0; for q in [V(1), W(9)] {{ r = q + q; t = t * 100 + (r is V ? r.x : r.y) }}; t"), 200);
	is!(&format!("{classes}t = 0; for n in [1, 2] {{ t = t + n + n }}; t"), 6);
}

#[test]
fn a_value_either_an_instance_or_a_number_keeps_what_it_is() {
	is!("class V{x:int}; r = if 1 > 0 then V(7) else 5; r.x", 7);
	is!("class V{x:int}; r = if 1 < 0 then V(7) else 5; r + 1", 6);
	is!("r = if 1 > 0 then {a:1} else 5; r.a", 1);
}

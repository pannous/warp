// A variable assigned a call of a function that returns a construction holds an instance of that class, so its
// operators and methods dispatch to the class (card units-dynamic: `q = quantity("5 km")`)
use crate::is;

const VECTOR: &str = "class V{x:int; plus(o) := V(x + o.x); times(k) := V(x * k)}\nmake(n) := V(n)\n";

#[test]
fn an_instance_returned_by_a_function_takes_its_class_operators() {
	is!(&format!("{VECTOR}a = make(1); b = make(2); (a + b).x"), 3);
	is!(&format!("{VECTOR}twice(n) := make(n * 2)\na = twice(1); (a * 5).x"), 10);
	is!(&format!("{VECTOR}parsed(t:text) := {{ n = t as int; V(n) }}\na = parsed(\"4\"); (a + make(1)).x"), 5);
}

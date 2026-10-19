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

#[test]
fn an_operation_of_instances_is_an_instance_too() {
	is!(&format!("{VECTOR}c = make(1) + make(2); (c * 2).x"), 6);
	is!(&format!("{VECTOR}a = make(1); c = a + a; d = c + a; d.x"), 3);
}

// a function's parameter named like a variable holding an instance is its own value
#[test]
fn a_parameter_shadows_an_instance_variable() {
	is!(&format!("{VECTOR}a = make(1)\nf(a) := a + 1\nf(2) + a.x"), 4);
}

// `make(n): V := …` declares its class: a call of it takes the class operators, `<` its less method (card uncertain-warp)
#[test]
fn a_declared_result_class_takes_its_class_operators() {
	let ordered = "class V{x:int; less(o:V):bool := x < o.x}\nmake(n:int): V := V(n)\n";
	is!(&format!("{ordered}make(1) < make(2)"), true);
	is!(&format!("{ordered}make(3) < make(2)"), false);
}

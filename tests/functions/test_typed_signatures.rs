// Several C-style typed parameters `f(int x, float y)`, as the single `f(int x)` already works
use crate::is;

#[test]
fn several_typed_parameters() {
	is!("f(int x, float y) := x+y; f(2, 0.5)", 2.5);
	is!("fun addier(int a, int b){b+a}; addier(42,1)+1", 44);
	is!("fun addi(float x, float y){x+y}; addi(2.2,2.2)", 4.4);
	is!("fun addier(float a, float b){b+a}; addier(42,1)+1", 44.0);
}

#[test]
#[ignore = "cleanup-edge: cleanup-park-phrases: type-name parameters (`fib int i`) dropped"]
fn spaced_typed_signature() {
	is!("f int x := x+1; f(2)", 3);
}

#[test]
fn a_parameter_of_a_type() {
	is!("f(x of int) := x+1; f(2)", 3);
	is!("f(n of number, m of number) := n+m; f(1,2)", 3);
	is!("f(s of text) := count(s); f(\"abc\")", 3);
}

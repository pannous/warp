// Several C-style typed parameters `f(int x, float y)`, as the single `f(int x)` already works
use warp::is;

#[test]
fn several_typed_parameters() {
	is!("f(int x, float y) := x+y; f(2, 0.5)", 2.5);
	is!("fun addier(int a, int b){b+a}; addier(42,1)+1", 44);
	is!("fun addi(float x, float y){x+y}; addi(2.2,2.2)", 4.4);
	is!("fun addier(float a, float b){b+a}; addier(42,1)+1", 44.0);
}

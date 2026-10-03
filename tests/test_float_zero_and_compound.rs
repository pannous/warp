// A compound assignment to a float variable inside a loop body stays float arithmetic, and `float[n]` holds float zeros.
use warp::wasm_emitter::eval;

#[test]
fn test_float_compound_assignment_in_a_loop() {
	assert_eq!(eval("s=0.0f; for x in [1.5f, 2.5f] {s+=x}; s"), eval("4.0f"));
	assert_eq!(eval("s=1.0f; for x in [2.0f, 3.0f] {s*=x}; s"), eval("6.0f"));
	assert_eq!(eval("s=0.0f; i=0; while i<3 {s+=0.5f; i++}; s"), eval("1.5f"));
	assert_eq!(eval("s=0.0f; x=1.5f; s+=x; s"), eval("1.5f"));
}

#[test]
fn test_float_zero_fill_holds_floats() {
	assert_eq!(eval("xs=float[3]; xs"), eval("[0.0f, 0.0f, 0.0f]"));
	assert_eq!(eval("xs=float[3]; xs[1]=1.5f; xs"), eval("[0.0f, 1.5f, 0.0f]"));
	assert_eq!(eval("xs : 2 float; xs"), eval("[0.0f, 0.0f]"));
	assert_eq!(eval("xs=float[3]; xs#1"), eval("0.0f"));
	assert_eq!(eval("xs=int[2]; xs"), eval("[0, 0]"));
}

use crate::is;

// A libm function called on a run-time value without `import … from 'm'` links libm by itself; it compiled to its
// argument before: `def f(x:float) := exp(x); f(0.0)` was 0, sin(0.5) in a function 0.5 (samples/neural_net.wasp sigmoid)

#[test]
fn test_libm_in_a_function_without_import() {
	is!("def f(x:float) := exp(x); f(0.0)", 1.0);
	is!("def f(x:float) := cos(x); f(0.0)", 1.0);
	is!("def f(x:float) := log(x); f(1.0)", 0.0);
	is!("def f(x:float) := fabs(x); f(-2.5)", 2.5);
	is!("def sigmoid(x) := 1.0 / (1.0 + exp(-x)); sigmoid(0.0)", 0.5);
}

#[test]
fn test_libm_on_a_runtime_value_without_import() {
	is!("x=random()*0; exp(x)", 1.0);
	is!("x=random()*0; sin(x)", 0.0);
}

#[test]
fn test_user_function_named_like_libm_wins() {
	is!("def sin(x) := 42; sin(1.0)", 42);
}

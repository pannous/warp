use crate::is;

#[test]
fn test_rounding_builtins_work_inside_function_bodies() {
	is!("f(x:float) := floor(x); f(2.5)", 2);
	is!("f(x:float) := ceil(x); f(2.5)", 3);
	is!("f(x:float) := round(x); f(2.5)", 2);
	is!("f(x:float) := floor(x) + 1; f(2.5)", 3);
	is!("g(x) := floor(x * 2); g(1.75)", 3);
}

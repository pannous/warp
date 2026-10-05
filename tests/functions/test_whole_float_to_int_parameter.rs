// P49b (user, 2026-10-05): a whole float passed to an int parameter is refused too, like `x:int = 2.0`
use crate::common::fails_with;
use warp::is;

#[test]
fn a_whole_float_for_an_int_parameter_is_refused() {
	fails_with("f(x:int) := x+1; f(2.0)", "is no int: write 2.0 as int");
	fails_with("f(x:int) := x+1; y=3.0; f(y)", "y is no int: write y as int");
	is!("f(x:int) := x+1; f(2.0 as int)", 3);
	is!("f(x:float) := x+1; f(2.0)", 3);
}

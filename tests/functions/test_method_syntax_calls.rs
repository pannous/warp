use crate::is;

// samples/neural_net.warp `output.round(3)`: a called method x.f(args) on a value is the call f(x, args) when f is a
// rounding or libm function; was "undefined function: round". User functions: test_user_method_form

#[test]
fn test_method_syntax_calls_a_rounding_function() {
	is!("x=2.5; x.floor()", 2);
	is!("x=2.4; x.round()", 2);
}

#[test]
fn test_method_syntax_calls_a_libm_function() {
	is!("import sin from 'm'; def f(x:float):=x.sin(); f(0.5)", 0.479425538604203);
	is!("x=0.0; x.sin()", 0.0);
	is!("x=0.0; x.cos()", 1.0);
}

use warp::is;

// samples/neural_net.wasp `output.round(3)`: a called method x.f(args) on a value is the call f(x, args) when f is a
// function (a user function, a rounding function or a libm function); was "undefined function: round"

#[test]
fn test_method_syntax_calls_a_user_function() {
	is!("def sq(x):=x*x; y=3; y.sq()", 9);
	is!("def add(a, b):=a+b; y=3; y.add(4)", 7);
}

#[test]
fn test_method_syntax_calls_a_rounding_function() {
	is!("x=2.5; x.floor()", 2);
	is!("x=2.4; x.round()", 2);
}

#[test]
fn test_method_syntax_calls_a_libm_function() {
	is!("x=0.0; x.sin()", 0.0);
	is!("x=0.0; x.cos()", 1.0);
}

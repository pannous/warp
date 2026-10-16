//! card codepoint-loop-variable: a loop variable or parameter named like a library word (`codepoint`, `ord`, `upper`)
//! is that value in its body, also read bare (`x = ord`), not the library function
use crate::is;

#[test]
fn a_bound_name_shadows_the_library_word() {
	is!("xs = []; for each codepoint in \"ab\" { x = codepoint; xs = xs + [x] }; xs", warp::parse("['a' 'b']"));
	is!("n = 0; for ord in [1 2] { x = ord; n = n + x }; n", 3);
	is!("f(upper) := { x = upper; x }; f(2)", 2);
}

#[test]
fn a_loop_variable_shadows_words_taking_nothing() {
	is!("for random in [3] { x = random; print x }; x", 3);
	is!("for now in [3] { x = now }; x", 3);
	is!("f() := 7; for f in [3] { x = f }; x", 3);
}

#[test]
fn operator_words_and_constants_bind_no_name() {
	let refused = |code: &str, message: &str| match warp::wasm_emitter::eval(code) {
		warp::Node::Error(reason) => assert!(format!("{reason:?}").contains(message), "{code}: {reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	};
	refused("for sqrt in [4] { print sqrt }", "sqrt is an operator (√); rename your variable");
	refused("for each abs in [4] { y = abs }", "abs is an operator");
	refused("g(sqrt) := [sqrt]; g(3)", "sqrt is an operator (√); rename your parameter");
	refused("sqrt = 5; sqrt", "rename your variable");
	refused("for pi in [3] { x = pi }", "pi is a constant");
}

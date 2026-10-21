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

// card rust-lane-loop: a variable or loop variable named like a library word is that value before a word operator
// too, at the top level and in every function (`first contains "a"` is no call first(contains "a"))
#[test]
fn a_bound_name_before_a_word_operator_is_its_value() {
	is!("first = [\"a\"]; first contains \"a\"", true);
	is!("g(first) := first contains \"a\"; g([\"a\"])", true);
	is!("g(upper: list) := upper has \"a\"; g([\"a\"])", true);
	is!("for words in [[1]] { print words }; def g(words: any): bool := words contains \"a\"; g([\"a\"])", true);
}

#[test]
fn probe_prefix_calls_split() {
	let shown: Vec<String> = ["g(first) := first contains \"a\"", "first = [\"a\"]; first contains \"a\""]
		.iter()
		.map(|code| format!("{code} => {:#?}", warp::pipeline::pass_trace(warp::parse(code))))
		.collect();
	panic!("PROBE {shown:?}");
}

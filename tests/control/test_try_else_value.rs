// User decision (notes/open_decisions.md, 2026-10-03): "try X else otherValueOrAction. I never invented the => Y
// syntax." `try X else Y` has no named form: a fallback written `e => …` is the lambda it is, a function value,
// never a binding of the caught error
use crate::is;
use warp::wasm_emitter::eval;

const CAUGHT_MESSAGE: &str = "index out of range";

#[test]
fn an_arrow_fallback_is_a_function_value() {
	for code in ["try 1 + [1 2]#5 else e => e", "try 1 + [1 2]#5 else (e => e * 2)"] {
		let value = eval(code);
		assert!(!matches!(value, warp::Node::Error(_)), "{code}: {value:?}");
		assert_ne!(value, warp::Node::Text(CAUGHT_MESSAGE.to_string()), "{code}");
		assert!(value.serialize().contains("lambda"), "{code}: the fallback is the function it is, got {value:?}");
	}
}

#[test]
fn try_else_takes_a_value_or_an_action() {
	is!("try 1 + [1 2]#5 else 0", 0);
	is!("x = 1; try 1 + [1 2]#5 else {x = 5}; x", 5);
	is!("try 1 + [1 2]#2 else 0", 3);
}

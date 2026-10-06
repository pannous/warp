//! P165 (user): hard keywords (control flow, declarations, literal values, modules) are never redefined; a soft keyword
//! (emit, send, init, new, root, listeners, every, …) may name a local variable, parameter, field or method, where the
//! program's name wins with a got-it note; redefining one at the top level is a loud error
use crate::common::fails_with;
use crate::is;

fn noted(code: &str, word: &str) -> warp::Node {
	let (value, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	assert!(hints.iter().any(|hint| hint.original == word && hint.reason.contains("is a keyword; here it is your")), "{hints:?}");
	value
}

#[test]
fn a_soft_keyword_names_a_local_with_a_note() {
	assert_eq!(noted("f(send) := send + 1; f(2)", "send"), 3);
	assert_eq!(noted("f(x) := { root = x + 1; root }; f(3)", "root"), 4);
	assert_eq!(noted("o = {every: 3}; o.every", "every"), 3);
}

#[test]
fn a_soft_keyword_redefined_at_the_top_level_is_an_error() {
	fails_with("send = 3; send", "send is a keyword");
	fails_with("root(x) := x * 3; root(2)", "root is a keyword");
	fails_with("def init() { 1 }; init()", "init is a keyword");
}

#[test]
fn a_hard_keyword_is_never_redefined() {
	fails_with("import = 4", "import is a keyword");
	fails_with("def = 2", "def is a keyword");
	fails_with("f(x) := { return = 2; x }; f(1)", "return is a keyword");
	is!("f(x) := { y = x + 1; y }; f(1)", 2);
}

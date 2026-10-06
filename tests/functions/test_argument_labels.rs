// Swift argument labels: `func greet(person name: String)` is called `greet(person: "Bob")` and reads `name` in its
// body; `_ name` takes the argument without a label
use crate::is;

#[test]
fn a_label_names_the_argument_and_the_name_is_read_inside() {
	is!("func greet(person name: String) -> String { \"Hi \" + name }; greet(person: \"Bob\")", "Hi Bob");
	is!("func move(from a: Int, to b: Int) -> Int { b - a }; move(from: 2, to: 9)", 7);
	is!("func move(from a: Int, to b: Int) -> Int { b - a }; move(2, 9)", 7);
}

#[test]
fn an_underscore_label_takes_the_argument_without_a_name() {
	is!("func twice(_ x: Int) -> Int { x * 2 }; twice(4)", 8);
	is!("func add(_ a: Int, _ b: Int) -> Int { a + b }; add(2, 3)", 5);
}

#[test]
fn a_labeled_parameter_of_a_function_type() {
	is!("func apply(_ f: (Int) -> Int, _ x: Int) -> Int { f(x) }; apply({ $0 + 1 }, 2)", 3);
	is!("func apply(using f: (Int) -> Int, to x: Int) -> Int { f(x) }; apply(using: { $0 * 3 }, to: 2)", 6);
}

#[test]
fn a_label_compiles_with_a_note_to_name_the_parameter_once() {
	// user 2026-10-06: "Swift's labeled parameter: we don't do this over here, I don't like that redundancy"
	let (value, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("func twice(_ x: Int) -> Int { x * 2 }; twice(4)"));
	assert_eq!(value, warp::Node::int(8));
	assert!(hints.iter().any(|hint| hint.reason.contains("names a parameter once")), "{hints:?}");
}

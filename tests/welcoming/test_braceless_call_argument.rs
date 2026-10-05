// P68 (user, 2026-10-05): a braceless call whose argument holds an operator (`square 3 + square(4)`) keeps its reading,
// square(3 + square(4)), with a got-it warning naming both readings
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;
use warp::Node;

fn warnings_of(code: &str) -> (Node, Vec<String>) {
	take_warnings();
	let value = eval(code);
	(value, take_warnings().iter().map(|warning| warning.to_string()).collect())
}

#[test]
fn an_operator_in_a_braceless_argument_warns_with_both_readings() {
	let (value, warnings) = warnings_of("square := it²; square 3 + square(4)");
	assert_eq!(value, Node::int(361));
	assert!(warnings.iter().any(|warning| warning.contains("square(3) + square(4)") && warning.contains("square(3 + square(4))")), "{warnings:?}");
}

#[test]
fn a_plain_braceless_argument_does_not_warn() {
	let (value, warnings) = warnings_of("square := it²; square 3");
	assert_eq!(value, Node::int(9));
	assert!(warnings.iter().all(|warning| !warning.contains("square(")), "{warnings:?}");
	let (_, warnings) = warnings_of("square := it²; square(3 + 1)");
	assert!(warnings.iter().all(|warning| !warning.contains("square(3) +")), "{warnings:?}");
}

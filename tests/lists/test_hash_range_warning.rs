// P58 (user, 2026-10-05: "create a strong warning and I don't care how to interpret it"): `xs#a..b` stays the range from
// the value xs#a, with a warning naming the slice `xs#(a..b)` and the range `(xs#a)..b`
use warp::is;

fn warnings_of(code: &str) -> (warp::Node, Vec<String>) {
	warp::diagnostic::take_warnings();
	let value = warp::wasm_emitter::eval(code);
	(value, warp::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect())
}

#[test]
fn an_unparenthesized_hash_range_warns() {
	let (value, warnings) = warnings_of("xs=[2,5]; r = xs#1..6; count(r)");
	assert_eq!(value, warp::Node::int(4));
	assert!(warnings.iter().any(|warning| warning.contains("xs#(1..6)") && warning.contains("(xs#1)..6")), "{warnings:?}");
}

#[test]
fn parenthesized_forms_do_not_warn() {
	let (_, warnings) = warnings_of("xs=[2,5]; r = (xs#1)..6; count(r)");
	assert!(warnings.iter().all(|warning| !warning.contains("xs#(")), "{warnings:?}");
	is!("xs=[1,2,3,4,5]; xs#(2…4)", warp::ints(vec![2, 3, 4]));
}

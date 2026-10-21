// P58 (user, 2026-10-05: "create a strong warning and I don't care how to interpret it"): `xs#a..b` stays the range from
// the value xs#a, with a warning naming the slice `xs#(a..b)` and the range `(xs#a)..b`
use crate::is;

fn warnings_of(code: &str) -> (warp::Node, Vec<String>) {
	warp::errors::diagnostic::take_warnings();
	let value = warp::wasm_emitter::eval(code);
	(value, warp::errors::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect())
}

#[test]
fn an_unparenthesized_hash_range_warns() {
	let (value, warnings) = warnings_of("xs=[2,5]; r = xs#1..6; count(r)");
	assert_eq!(value, warp::Node::int(4));
	assert!(
		warnings.iter().any(|warning| warning.contains("xs#(1..6)") && warning.contains("(xs#1)..6")),
		"{warnings:?}"
	);
}

#[test]
fn parenthesized_forms_do_not_warn() {
	let (_, warnings) = warnings_of("xs=[2,5]; r = (xs#1)..6; count(r)");
	assert!(warnings.iter().all(|warning| !warning.contains("xs#(")), "{warnings:?}");
	is!("xs=[1,2,3,4,5]; xs#(2…4)", warp::ints(vec![2, 3, 4]));
}

/// `name[#p..]`, `#xs down to 1`: a count `#p` before a range is no element `xs#a`, so no warning (card
/// prefix-count-warning)
#[test]
fn a_prefix_count_before_a_range_does_not_warn() {
	let (value, warnings) = warnings_of("name=\"no field x\"; p=\"no field \"; name[#p..]");
	assert_eq!(value, warp::Node::text("x"));
	assert!(warnings.iter().all(|warning| !warning.contains("range from the value")), "{warnings:?}");
	let (_, warnings) = warnings_of("xs=[1,2,3]; n = 0; for i in #xs down to 1 { n += i }; n");
	assert!(warnings.iter().all(|warning| !warning.contains("range from the value")), "{warnings:?}");
}

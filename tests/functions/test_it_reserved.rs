// card it-reserved (user, 2026-10-16: "we should create a warning that it is a reserved keyword"): `it` names the
// parameter of a function that names none, so assigning it warns; the program still runs (an error under use strict)
use crate::is;

fn warnings_of(code: &str) -> (warp::Node, Vec<String>) {
	warp::errors::diagnostic::take_warnings();
	let value = warp::wasm_emitter::eval(code);
	(value, warp::errors::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect())
}

#[test]
fn assigning_it_warns_that_it_is_reserved() {
	let (value, warnings) = warnings_of("it = 5; it + 1");
	assert_eq!(value, warp::Node::int(6));
	assert!(warnings.iter().any(|warning| warning.contains("`it` is reserved")), "{warnings:?}");
	let (_, warnings) = warnings_of("square := it * it; square(3)");
	assert!(!warnings.iter().any(|warning| warning.contains("reserved")), "{warnings:?}");
	is!("square := it * it; square(3)", 9);
	let strict = warp::wasm_emitter::eval("use strict; it = 5; it").serialize();
	assert!(strict.contains("`it` is reserved"), "{strict}");
}

//! The hint toward `as` for a `->` result type points at the user's definition, not at a library line (return-as)
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

#[test]
fn the_arrow_hint_points_at_the_definition() {
	let (_, hints) = capture_hints(|| eval("def half(x) -> int { x / 2 }"));
	let hint = hints.iter().find(|hint| hint.reason.contains("as int")).expect("a hint toward as");
	assert_eq!(hint.line_and_column().0, 1, "{}", hint.position);
}

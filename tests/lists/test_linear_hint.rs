// card linear-hint: 'prefer xs = float[n] over linear' only where the compiler would pick linear memory itself, which
// it does for float arrays paired by dot and `.*` (card compiler-picks-dot)
use warp::normalize::{capture_hints, clear_shown_hints};

fn linear_hinted(code: &str) -> bool {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().any(|hint| hint.original.starts_with("linear"))
}

// the compiler picks linear memory for float arrays paired by dot or `.*` itself (card compiler-picks-dot)
#[test]
fn linear_arrays_of_dot_are_hinted_too() {
	assert!(linear_hinted("linear xs = float[3]; linear ys = float[3]; dot(xs, ys)"));
	assert!(linear_hinted("linear xs = float[3]; linear ys = float[3]; sum(xs .* ys)"));
	assert!(linear_hinted("linear xs = float[3]; zs = xs .* xs; zs#1"));
}

#[test]
fn a_linear_array_of_plain_indexing_is_hinted() {
	assert!(linear_hinted("linear xs = int[3]; xs#1 = 2; xs#1"));
}

//! Card easy-add (user: "arrow down for logarithm"): `x↓b` is log base b of x, and `x↑n`, Knuth's up arrow, is x^n;
//! a glyph that is already the operator's spelling gets no note
use crate::is;

#[test]
fn the_down_arrow_is_the_log_to_a_base() {
	is!("8↓2", 3);
	is!("100↓10", 2);
	is!("x = 8; x↓2 + 1", 4);
}

#[test]
fn the_up_arrow_is_the_power() {
	is!("2↑3", 8);
	is!("(2↑3)↓2", 3);
}

#[test]
fn a_canonical_glyph_gets_no_note() {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("8↓2"));
	assert!(!hints.iter().any(|hint| hint.original == hint.canonical), "{hints:?}");
}

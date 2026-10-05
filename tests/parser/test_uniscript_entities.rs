// Uniscript entities in code (wiki/uniscript.md, unicode.md, Features.md): `\:name` and `\name` are the character
use warp::wasp_parser::parse;
use warp::{eq, is};

#[test]
fn entities_are_their_characters() {
	is!("\\alpha = 4; α", 4);
	is!("α = 4; \\alpha + 1", 5);
	is!("\\:alpha = 2; \\alpha * 3", 6);
	eq!(parse("\\:infinity"), parse("∞")); // ∞ itself is no value yet (todo.md)
	is!("\\pi == π", 1);
	is!("3 \\leq 4", 1);
	is!("\\Delta = 7; Δ", 7);
}

#[test]
fn entities_stay_text_inside_quotes_and_comments() {
	is!("\"a\\nat\"", "a\nat"); // a newline, not ℕ
	is!("x = 1 // \\alpha\nx", 1);
}

#[test]
fn an_unknown_entity_is_an_error() {
	crate::common::fails_with("\\alphabet = 1", "\\alphabet");
}

// Uniscript entities (wiki/uniscript.md, unicode.md, Features.md). P56 (user, 2026-10-05): the syntax is `\:name`, in code
// and inside double-quoted texts; a bare `\name` is no entity (`"\nat"` stays a newline); an unknown entity is loud
use crate::is;

#[test]
fn entities_are_their_characters() {
	is!("\\:alpha = 4; α", 4);
	is!("α = 4; \\:alpha + 1", 5);
	is!("\\:alpha = 2; \\:alpha * 3", 6);
	is!("\\:infinity == ∞", 1);
	is!("\\:pi == π", 1);
	is!("3 \\:leq 4", 1);
	is!("\\:Delta = 7; Δ", 7);
	is!("\\:epsilon == ε", 1);
}

#[test]
fn entities_expand_inside_double_quoted_texts() {
	is!("\"x is \\:alpha\"", "x is α");
	is!("\"a\\nat\"", "a\nat"); // a newline, not ℕ
	is!("x = 1 // \\:alpha\nx", 1);
}

#[test]
fn an_unknown_entity_is_an_error() {
	crate::common::fails_with("\\:alphabet = 1", "\\:alphabet");
	crate::common::fails_with("\\alpha = 1", "\\:alpha");
}

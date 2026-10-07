// Uniscript entities (wiki/uniscript.md, unicode.md, Features.md). P56 (user, 2026-10-05): the syntax is `\:name`, in code
// and inside double-quoted texts; a bare `\name` is no entity (`"\nat"` stays a newline); an unknown entity warns (card unknown-entity)
use crate::is;
use warp::diagnostic::{with_warning_mode, WarningMode};

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
	let (value, warnings) = crate::common::warnings_of("\\:alphabet = 1");
	assert!(!matches!(value, warp::Node::Error(_)) && warnings.iter().any(|warning| warning.contains("\\:alphabet")), "{value:?} {warnings:?}");
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("\\:alphabet = 1", "\\:alphabet")); // strict
	crate::common::fails_with("\\alpha = 1", "\\:alpha");
}

// card unknown-entity (user, 2026-10-07: an unknown entity must not stop the program): `\:world` stays as written and
// warns, in a text and outside one
#[test]
fn an_unknown_entity_stays_as_written_and_warns() {
	crate::common::warns_with("\"hello \\:world:\"", "hello \\:world:", "unknown entity \\:world");
	crate::common::warns_with("\"\\:alpha and \\:world\"", "α and \\:world", "unknown entity \\:world");
	crate::common::warns_with("x = \\:world; x", "\\:world", "unknown entity \\:world");
}

// P126 (user): texts inside containers are quoted everywhere (print, interpolation, string()): ["a" "b"],
// Point{x:1 name:"a"}; a text on its own still prints without quotes
#[cfg(feature = "native")] // printed runs the warp binary
use crate::common::printed;
use crate::is;

#[cfg(feature = "native")]
#[test]
fn a_text_in_a_list_prints_quoted() {
	assert_eq!(printed("print [\"a\" \"b\"]"), "[\"a\" \"b\"]\n");
	assert_eq!(printed("xs = [1, \"b\"]; print xs"), "[1 \"b\"]\n");
	assert_eq!(printed("print [[\"a\"]]"), "[[\"a\"]]\n");
	assert_eq!(printed("print \"a\""), "a\n");
	assert_eq!(printed("xs = [\"a\"]; print xs#1"), "a\n");
}

#[cfg(feature = "native")]
#[test]
fn a_text_field_prints_quoted() {
	assert_eq!(printed("class Point{x:int name:text}; p = Point{x:1 name:\"a\"}; print p"), "Point{x:1 name:\"a\"}\n");
	assert_eq!(printed("m = {a:\"x\" b:2}; print m"), "{a:\"x\" b:2}\n");
}

#[test]
fn string_and_interpolation_quote_texts_in_containers() {
	is!("xs = [\"a\", \"b\"]; string(xs)", "[\"a\" \"b\"]");
	is!("string([\"a\", \"bc\"])", "[\"a\" \"bc\"]");
	is!("xs = [\"a\", \"b\"]; \"got \\(xs)\"", "got [\"a\" \"b\"]");
	is!("x = \"a\"; \"got \\(x)\"", "got a");
	is!("class Point{x:int name:text}; p = Point{x:1 name:\"a\"}; string(p)", "Point{x:1 name:\"a\"}");
}

// P126 follow-up: a quote or backslash inside a quoted text is escaped, so the text reads back as written
#[test]
fn a_quote_inside_a_text_in_a_container_is_escaped() {
	is!("string([\"say \\\"hi\\\"\"])", "[\"say \\\"hi\\\"\"]");
	is!("string([\"a\\\\b\"])", "[\"a\\\\b\"]");
	is!("xs = [\"q\\\"\"]; \"got \\(xs)\"", "got [\"q\\\"\"]");
	is!("class P{name:text}; p = P{name:\"a\\\"b\"}; string(p)", "P{name:\"a\\\"b\"}");
}

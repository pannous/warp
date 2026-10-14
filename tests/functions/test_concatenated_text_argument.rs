//! A text built by concatenation is a text argument: the parameter it fills is no list
use crate::is;

#[test]
fn concatenated_text_fills_a_counted_parameter() {
	is!("p(t, w) := (w - count(t)) times \" \" + t; p(\"\" + 7, 3)", "  7");
	is!("use text; s = \"\" + 7; pad_left(s, 3)", "  7");
	is!("use text; \"x\" + pad_left(\"#\" + 7, 3)", "x #7");
}

#[test]
fn text_returned_by_a_call_fills_a_counted_parameter() {
	is!("p(t) := (4 - count(t)) times \" \" + t; q(t) := t + \"!\"; p(q(\"a\"))", "  a!");
	is!("use text; \"[\" + pad_right(pad_left(\"ab\", 4), 6) + \"]\"", "[  ab  ]");
}

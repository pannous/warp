//! A text built by concatenation is a text argument: the parameter it fills is no list
use crate::is;

#[test]
fn concatenated_text_fills_a_counted_parameter() {
	is!("p(t, w) := (w - count(t)) times \" \" + t; p(\"\" + 7, 3)", "  7");
	is!("use text; s = \"\" + 7; pad_left(s, 3)", "  7");
	is!("use text; \"x\" + pad_left(\"#\" + 7, 3)", "x #7");
}

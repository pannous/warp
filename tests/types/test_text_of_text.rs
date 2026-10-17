// The text of a converted value is the text of that value, not its source: `text(text(5))` is "5" (was "(text 5)")
use crate::is;

#[test]
fn text_of_a_text_conversion_is_its_value() {
	is!("text(text(5))", "5");
	is!("text text 5", "5");
	is!("text(str(5))", "5");
	is!("str(text(5))", "5");
	is!("text(5 as text)", "5");
	is!("(5 as text) as text", "5");
	is!("text(hex(255))", "0xFF");
}

#[test]
fn text_of_data_stays_its_source() {
	is!("text(data x+1)", "x+1");
}

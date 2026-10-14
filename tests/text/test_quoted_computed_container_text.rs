use crate::is;

#[cfg(feature = "native")]
use crate::common::printed;

const COMPUTED_QUOTED_TEXT: &str = "quote = \"\\\"\"; quoted = quote + \"ab\" + quote";

#[test]
fn a_computed_quote_inside_a_list_text_is_escaped() {
	is!(&format!("{COMPUTED_QUOTED_TEXT}; string([quoted])"), "[\"\\\"ab\\\"\"]");
}

#[cfg(feature = "native")]
#[test]
fn printing_a_computed_quote_inside_a_list_is_escaped() {
	assert_eq!(printed(&format!("{COMPUTED_QUOTED_TEXT}; print [quoted]")), "[\"\\\"ab\\\"\"]\n");
}

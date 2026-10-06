use crate::is;

#[test]
fn test_property_word_after_a_name_counts() {
	for word in ["size", "count", "length", "number"] {
		is!(&format!("pixels=[1 2 4];pixels {word}"), 3);
	}
}

#[test]
fn test_property_word_agrees_with_of_and_getter() {
	for word in ["size", "count", "length", "number"] {
		is!(&format!("pixels=[1 2 4];{word} of pixels"), 3);
		is!(&format!("pixels=[1 2 4];pixels.{word}"), 3);
	}
}

#[test]
fn test_property_word_counts_text_characters() {
	is!("t=\"héllo\";t size", 5);
}

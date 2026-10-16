use crate::is;

#[test]
fn test_property_word_agrees_with_size_of() {
	is!("pixels=[1 2 4];pixels size", 3);
}

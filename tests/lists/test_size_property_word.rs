use crate::is;

#[test]
fn test_property_word_after_a_name_counts_like_dot_property() {
	is!("pixel=[1 2 4];pixel size", 3);
	is!("pixel=[1 2 4];pixel length", 3);
	is!("pixel=[1 2 4];pixel count", 3);
	is!("pixel=[1 2 4];pixel number", 3);
}

#[test]
fn test_property_word_agrees_with_size_of() {
	is!("pixels=[1 2 4];pixels size", 3);
	is!("t=\"héllo\";t size", 5);
}

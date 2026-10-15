use crate::is;

// card chars-chars: a variable named like a counting unit (chars, bytes) is that variable before `in`

#[test]
fn a_variable_named_like_a_unit_is_the_variable() {
	is!("chars = [\"a\"]; \"a\" in chars", true);
	is!("bytes = [1 2]; 3 in bytes", false);
}

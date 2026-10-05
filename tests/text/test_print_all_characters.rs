// `print all characters in "hello"` (wiki sweep): prints each character, like `print chars in "hello"`; `characters` is
// a text unit like `chars`
#![cfg(feature = "native")]

#[test]
fn print_all_characters_prints_each() {
	assert!(crate::common::printed("print all characters in \"hi\"").starts_with("h\ni\n"));
	assert!(crate::common::printed("print characters in \"hi\"").starts_with("h\ni\n"));
}

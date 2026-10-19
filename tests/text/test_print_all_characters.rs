// `print all characters in "hello"` (wiki sweep): prints each character, like `print chars in "hello"`; `characters` is
// a text unit like `chars`
#![cfg(feature = "native")]

#[test]
#[ignore = "dropped: rust-cleanup-drops: the walk print words in t; warp writes for w in words in t: print w"]
fn print_all_characters_prints_each() {
	assert!(crate::common::printed("print all characters in \"hi\"").starts_with("h\ni\n"));
	assert!(crate::common::printed("print characters in \"hi\"").starts_with("h\ni\n"));
}

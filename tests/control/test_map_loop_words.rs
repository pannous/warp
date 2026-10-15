// card key-print: `for key in m print key`, the words after a map are the loop's body, as after a list
#[cfg(feature = "native")]
use crate::common::printed;

#[cfg(feature = "native")]
#[test]
fn the_words_after_a_map_are_the_body() {
	assert_eq!(printed("for key in {a: 1, b: 2} print key"), "a\nb\n");
	assert_eq!(printed("m = {a: 1, b: 2}; for key in m print key"), "a\nb\n");
	assert_eq!(printed("m = {a: 1, b: 2}; for k, v in m print v"), "1\n2\n");
}

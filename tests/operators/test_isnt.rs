//! `isn't` (straight or curly apostrophe) is `is not`: `3 isn't 4` is yes (wiki/ambiguity.md "is not")
use crate::is;

#[test]
fn test_isnt_spellings() {
	for spelling in ["is not", "isn't", "isn’t"] {
		is!(&format!("3 {spelling} 4"), true);
		is!(&format!("3 {spelling} 3"), false);
		is!(&format!("x = 'a'; x {spelling} 'b'"), true);
	}
}

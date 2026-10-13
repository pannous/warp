//! Card matches-infix: an infix operator a standard module declares is infix where the module is used,
//! `use regex; "abc" matches "a.c"` (lib/regex.warp declares `infix operator matches`)
use crate::is;

#[test]
fn a_used_module_declares_an_infix_word() {
	is!("use regex; \"abc\" matches \"a.c\"", true);
	is!("use regex; \"abd\" matches \"a.c$\"", false);
	is!("use regex; matches(\"abc\", \"a.c\")", true);
}

#[test]
fn the_word_stays_a_word_without_the_module() {
	is!("matches = 3; matches + 1", 4);
}

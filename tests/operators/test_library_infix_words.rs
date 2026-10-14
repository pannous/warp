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

/// Card use-regex: the word glued to `(` is its call, also after a command word: `print` is no left operand
#[test]
fn a_glued_call_of_an_infix_word_is_a_call() {
	is!("use regex; print matches(\"abd\", \"a.c$\")", false);
	is!("use regex; x = matches(\"abc\", \"a.c\"); x", true);
}

/// Card counts-parens: a parenthesis inside a text is no group bracket, the glued call stays a call
#[test]
fn a_parenthesis_in_a_text_keeps_the_call() {
	is!("use regex; print matches(\"(\", \"x\")", false);
	// print gives ø, which equals false loosely: the value itself tells yes from no
	is!("use regex; ok = matches(\"a)\", \"a\\\\)\"); ok", true);
	is!("use regex; ok = matches(\"(\", \"x\"); ok", false);
	is!("sqrt(\")\".count + 3).round(1)", 2);
}

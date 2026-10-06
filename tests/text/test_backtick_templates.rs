// JavaScript template literals (card text-backtick): `Hello ${name}` is the text "Hello ${name}", holes interpolated,
// newlines kept; a hint names the double quotes wasp writes it with
use crate::is;

#[test]
fn a_backtick_template_interpolates() {
	is!("name = \"Ann\"; `Hello ${name}`", "Hello Ann");
	is!("a = 2; `${a} + ${a} = ${a + a}`", "2 + 2 = 4");
}

#[test]
fn a_backtick_text_is_text() {
	is!("`plain words`", "plain words");
	is!("x = `a\nb`; #x", 3);
	is!("`it\\`s`", "it`s");
}

#[test]
fn an_unclosed_backtick_says_so() {
	crate::common::fails_with("x = `open", "no closing `");
}

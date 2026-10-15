//! A labelled argument named like a tag is an argument, no element (card labelled-a-argument): `f(a: g(2))` passes
//! g(2) as a, it renders no <a>; markup passed as an argument's value is still checked
use crate::common::warnings_of;

const NO_HREF: &str = "a without href";

fn href_warnings(code: &str) -> Vec<String> {
	warnings_of(code).1.into_iter().filter(|warning| warning.contains(NO_HREF)).collect()
}

#[test]
fn a_labelled_argument_named_like_a_tag_is_no_element() {
	assert_eq!(href_warnings("g(x) := x; f(a) := a; f(a: g(2))"), Vec::<String>::new());
	assert_eq!(href_warnings("f(a) := a; f(a: [1])"), Vec::<String>::new());
	assert_eq!(href_warnings("f(a, b) := b; f(1, b: max(2, 3))"), Vec::<String>::new());
}

#[test]
fn markup_passed_as_an_argument_is_still_checked() {
	assert_eq!(href_warnings("page(x) := x; page(a{\"home\"})").len(), 1);
	assert_eq!(href_warnings("page(content) := content; page(content: a{\"home\"})").len(), 1);
}

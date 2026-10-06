use crate::is;

#[test]
fn count_unit_of_text() {
	is!("count bytes of \"abc\"", 3);
	is!("count chars of \"abc\"", 3);
	is!("count bytes of \"äb\"", 3);
	is!("count chars of \"äb\"", 2);
	is!("x=\"äb\"; count bytes of x", 3);
}

#[test]
fn size_of_text_in_unit() {
	is!("size of \"äb\" in bytes", 3);
	is!("size of \"äb\" in chars", 2);
	is!("number of bytes in \"äb\"", 3);
}

use crate::is;

#[test]
fn test_size_is_element_count() {
	is!("pixels=(1,2,3);size(pixels)", 3);
	is!("pixels=[1 2 4];size of pixels", 3);
}

#[test]
fn test_size_of_text_is_character_count() {
	is!("size \"👍🏽\"", 1);
	is!("x=\"héllo\";x.size", 5);
	is!("t=\"héllo\";size of t", 5);
}

#[test]
fn test_bytes_need_explicit_unit() {
	is!("x=\"👍🏽\";count x.bytes", 8);
	is!("t=\"héllo\";byte count of t", 6);
	is!("byte count of \"héllo\"", 6);
	is!("t=\"héllo\";number of bytes in t", 6);
	is!("pixels=[1 2 4];#bytes in pixels", 24);
	is!("pixels=[1 2 4];pixels.bytecount", 24); // decision text-unit-plurals: .bytes is the byte list
	is!("pixels=[1 2 4];number of bytes in pixels", 24);
}

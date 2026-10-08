//! `use text`: to_utf8 gives a text's UTF-8 bytes, from_utf8 the text of bytes; Rust's String::from_utf8 (written
//! string.from_utf8) is from_utf8 with a note; a standard word works called as a method too (samples/wasm_interop.warp)
use crate::is;
use warp::ints;

#[test]
fn a_text_and_its_utf8_bytes() {
	is!("use text; to_utf8(\"hé€\")", ints(vec![104, 195, 169, 226, 130, 172]));
	is!("use text; from_utf8(to_utf8(\"hé€😀\"))", "hé€😀");
	is!("string.from_utf8([104, 105])", "hi");
	is!("use text; \"hé\".to_utf8().count", 3);
}

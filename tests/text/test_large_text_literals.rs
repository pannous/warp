//! Text literals beyond one memory page (64 KiB) fit: the module's memory starts as large as its texts (found by card
//! self-serializer: a program holding 100 KB of sources as texts trapped with "out of bounds memory access")
use crate::is;

const PAGE: usize = 65536;

#[test]
fn a_text_literal_larger_than_a_page_fits() {
	let size = PAGE + 4464;
	is!(&format!("x = \"{}\"\n#x", "a".repeat(size)), size as i64);
}

#[test]
fn text_literals_larger_than_a_page_together_fit() {
	let literals: Vec<String> = (0..20).map(|i| format!("#\"{}\"", "abcdefghij".repeat(500 + i))).collect();
	is!(&format!("[{}]#20", literals.join(", ")), 5190);
}

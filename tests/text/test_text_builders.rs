//! card self-byte-buffer: texts built side by side (a lexer's token text, its output, a comment) each grow in amortised
//! O(1): a text copied to grow gets room to grow into, so appends to several texts in turn stay linear
use crate::is;

/// Appends per text: quadratic copying would need 10^10 bytes and pass the memory cap
const APPENDS: i64 = 100_000;

#[test]
fn texts_built_in_turn_grow_linearly() {
	is!(
		&format!("w = \"\"; v = \"\"; for i in 0..{APPENDS} {{ w += \"x\"; v += \"yz\" }}; count(w) + count(v)"),
		3 * APPENDS
	);
	is!(
		&format!(
			"a = \"\"; b = \"\"; c = \"\"; for i in 0..{APPENDS} {{ a += \"a\"; b += \"é\"; c += a[i] }}; count(a) + count(b) + count(c)"
		),
		3 * APPENDS
	);
}

#[test]
fn a_text_grown_in_its_room_leaves_earlier_values() {
	let built = "a = \"\"; b = \"\"; for i in 0..100 { a += \"x\"; b += \"y\" }";
	is!(
		&format!("{built}; c = a; a += \"z\"; d = c + \"w\"; count(c) * 10000 + count(a) * 10 + count(d) - 101"),
		1001010
	);
	is!(
		&format!("{built}; c = a; a += \"z\"; d = c + \"w\"; c + \"|\" + a[100] + d[100]"),
		("x".repeat(100) + "|zw").as_str()
	);
	is!(&format!("{built}; s = a[90..]; s += \"q\"; a += \"r\"; s + a[100]"), ("x".repeat(10) + "qr").as_str());
}

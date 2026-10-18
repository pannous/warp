//! decision text-unit-plurals: every plural unit word of a text is the list of those units, like chars, words and
//! lines; counting is `count t.codepoints`, `number of codepoints in t`
use crate::is;

#[test]
fn codepoints_and_graphemes_are_lists() {
	is!("t = \"👍🏽\"; t.codepoints", warp::parse("['👍' '🏽']"));
	is!("t = \"👍🏽\"; t.graphemes", warp::parse("[\"👍🏽\"]"));
	is!("t = \"ab\"; t.codepoints.map(c => c + \"!\")", warp::parse("[\"a!\" \"b!\"]"));
	is!("t = \"שָלוֹם\"; codepoints in t", warp::parse("['ש' 'ָ' 'ל' 'ו' 'ֹ' 'ם']"));
}

#[test]
fn counting_phrases_count_the_unit_lists() {
	let shalom = |code: &str| format!("t = \"שָלוֹם\"; {code}");
	let codepoint_counts = [
		"count t.codepoints",
		"number of codepoints in t",
		"#codepoints in t",
		"count codepoints in t",
		"size of t in codepoints",
	];
	for six in codepoint_counts {
		is!(&shalom(six), 6);
	}
	for four in ["count t.graphemes", "count graphemes in t", "number of graphemes in t"] {
		is!(&shalom(four), 4);
	}
}

#[test]
fn bytes_are_the_byte_list_and_count_as_the_byte_size() {
	is!("t = \"é!\"; t.bytes", warp::parse("[195 169 33]"));
	is!("t = \"👍🏽\"; count t.bytes", 8);
	is!("t = \"👍🏽\"; t.bytecount", 8);
	is!("for byte in \"äb\" { byte }", 98);
	// any value's bytes count as its memory size without the list being made
	for twenty_four in ["pixels.bytecount", "count pixels.bytes", "pixels.bytes.count", "number of bytes in pixels"] {
		is!(&format!("pixels = [1 2 4]; {twenty_four}"), 24);
	}
}

#[test]
#[ignore = "dropped: rust-cleanup-drops: the unit count codepoint count of t; warp writes number of codepoints in t"]
fn a_unit_count_of_a_text() {
	is!("t = \"שָלוֹם\"; codepoint count of t", 6);
}

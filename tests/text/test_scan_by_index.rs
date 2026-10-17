//! Card self-text-index: `text[i]` in a loop takes constant time per step (a cursor remembers where the last index
//! landed), so a lexer may scan its source char by char instead of walking from the start at every index
use crate::is;

/// Text pieces of 4 graphemes, and the milliseconds a scan of them may take: well under, quadratic would take minutes
const PIECES: i64 = 25_000;
const MILLISECONDS: i64 = 1500;

fn timed_scan(piece: &str, condition: &str) -> String {
	format!("t = \"\"; for i in 0..{PIECES} {{ t += \"{piece}\" }}; w = \"xyz\"; n = t.length; s = 0; i = 0; t0 = clock(); \
		while i < n {{ if {condition} {{ s += 1 }}; i += 1 }}; if clock() - t0 < {MILLISECONDS} then s else -1")
}

#[test]
fn a_char_by_char_scan_takes_linear_time() {
	is!(&timed_scan("ab c", "t[i] == ' '"), PIECES);
	is!(&timed_scan("é👍🏽 c", "t[i] == ' '"), PIECES);
	// another text indexed in between, and a look back
	is!(&timed_scan("ab c", "t[i] == ' ' and w[i % 3] != 'q'"), PIECES);
	is!(&timed_scan("ab c", "i > 0 and t[i] == ' ' and t[i - 1] == 'b'"), PIECES);
}

#[test]
fn indexes_in_any_order_find_their_grapheme() {
	is!(
		"t = \"aé👍🏽\\r\\nb\"; o = \"\"; for i in [0 1 2 3 4 3 2 1 0 4 2] { o += t[i] + \"|\" }; o",
		"a|é|👍🏽|\r\n|b|\r\n|👍🏽|é|a|b|👍🏽|"
	);
	is!("t = \"ab\\r\\ncd\"; o = \"\"; for i in [4 3 2 1 0 2] { o += t[i] }; o", "dc\r\nba\r\n");
	// the same bytes grown or cut: an append in place, a prefix
	is!("t = \"abc\"; x = t[2]; t += \"de\"; t[2] + t[4]", "ce");
	is!("t = \"abcdef\"; x = t[4]; p = t[0..3]; [p.length, t[4]]", warp::parse("[3 'e']"));
}

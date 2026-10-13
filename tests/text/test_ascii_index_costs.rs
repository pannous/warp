// card text-costs: an ASCII character is a grapheme of its own unless a mark follows it, so text[i] steps over ASCII
// by the byte: a far index and a char-by-char scan of ASCII text take a few dozen steps per character, not hundreds
use crate::is;
use warp::util::with_fuel;

/// 20 000 characters: a far index walks them all, a scan reads each once
const SOURCE: &str = "code = \"abcdefghij\" * 2000\n";
/// ~1.2 M steps with the ASCII path, 6.2 M grapheme by grapheme
const FAR_BUDGET: u64 = 2_500_000;
/// ~7.4 M steps with the ASCII path, 17.4 M grapheme by grapheme
const SCAN_BUDGET: u64 = 11_000_000;

#[test]
fn ascii_text_indexes_by_the_byte() {
	with_fuel(FAR_BUDGET, || is!(&format!("{SOURCE}code[19990]"), 'a'));
	let scan = format!("{SOURCE}n = 0\nfor i in 0..19999 {{ if code[i] == 'a' {{ n += 1 }} }}\nn");
	with_fuel(SCAN_BUDGET, || is!(&scan, 2000));
	// a mark after an ASCII letter still joins it, and \r\n stays one grapheme
	is!("\"e\u{301}x\"[1]", 'x');
	is!("\"a\\r\\nb\"[2]", 'b');
}

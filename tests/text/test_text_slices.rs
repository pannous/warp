// A text slice t[a:b] is the graphemes a…b-1 sharing the text's bytes, found by the grapheme cursors: it used to turn
// the whole text into characters and join them back (card text-slice). The cursors and counts kept for a long text
// survive short texts read between its reads (card long-text). Texts built at run time (random) so nothing folds.
use crate::is;
use warp::util::with_fuel;

/// linear scans take a few million steps; each slice or index walking the 60 KB text again takes billions
const LINEAR_BUDGET: u64 = 100_000_000;

#[test]
fn a_text_slice_is_its_graphemes() {
	is!("t = \"héllo wörld\"; t[1:4]", "éll");
	is!("t = \"héllo\"; t[2:]", "llo");
	is!("t = \"héllo\"; t[:2]", "hé");
	is!("t = \"abc\"; t[1:99]", "bc");
	is!("t = \"héllo\"; t[3:1]", "");
	is!("t = \"a👍🏽b\"; t[1:2]", "👍🏽");
}

#[test]
fn consecutive_slices_of_a_long_text_take_linear_steps() {
	let slices = "t = \"abcdefghij\" * (6000 + round(random() * 0))\ntotal = 0\nfor i in 0 to 1999 { total += count(t[i * 30 : i * 30 + 5]) }\ntotal";
	with_fuel(LINEAR_BUDGET, || is!(slices, 10000));
}

#[test]
fn short_texts_read_between_leave_a_long_texts_cursor() {
	let lexer = "src = \"trüe,fälse,\" * (1000 + round(random() * 0))\nfirst = \"trüe\" * (1 + round(random() * 0))\nseparator = \",\" * (1 + round(random() * 0))\nhits = 0\nfor i in 1 to count(src) {\n\tletter = first#1\n\tcomma = separator#1\n\tif src#i == letter or src#i == comma { hits += 1 }\n}\nhits";
	with_fuel(LINEAR_BUDGET, || is!(lexer, 3000));
}

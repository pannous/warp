// card text-cache-lru: a longer text counted once, long ago (the largest file a parser read before), keeps no slot of the
// text counts and cursors: the text read now keeps its count while short words and a second long text are read by turns.
// It used to keep the slot with the larger count, so each turn recounted the text read now (billions of steps).
// Texts built at run time (random) so nothing folds.
use crate::is;
use warp::util::with_fuel;

/// ~60 M steps each with the counts kept, over 20 G recounting the source at every position
const LINEAR_BUDGET: u64 = 300_000_000;
const EARLIER: &str = "earlier = \"abcdefghij\" * (3000 + round(random() * 0))\ncounted = count(earlier)\n";
const SOURCE: &str = "src = \"abc def ghi \" * (1000 + round(random() * 0))\n";

#[test]
fn a_lexer_keeps_its_source_count_after_a_longer_text() {
	let starts_at = "def starts_at(source: any, part: text, at: int): bool {\n\ti = 0\n\twhile i < part.length and at + i < source.length and source[at + i] == part[i] { i = i + 1 }\n\ti == part.length\n}\n";
	let lexer = format!("{starts_at}{EARLIER}{SOURCE}words = [\"de\", \"gh\", \"zz\", \"def\"]\nhits = 0\nfor at in 0 to count(src) - 1 {{ for word in words {{ if starts_at(src, word, at) {{ hits += 1 }} }} }}\nhits");
	with_fuel(LINEAR_BUDGET, || is!(&lexer, 3000));
}

#[test]
fn two_long_texts_counted_by_turns_after_a_longer_one() {
	let turns = format!("{EARLIER}{SOURCE}line = \"x\" * (100 + round(random() * 0))\ntotal = 0\nat = 0\nwhile at < count(src) {{ total += count(line); at += 1 }}\ntotal");
	with_fuel(LINEAR_BUDGET, || is!(&turns, 1200000));
}

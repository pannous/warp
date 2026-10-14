//! Card runtime-speed: `for ch in text` takes linear time: the loop's `#text` each round counts the text once (the
//! text counted last keeps its count), and `text#i` walks on from the text cursor
use crate::is;

/// Pieces of 2 characters, and the milliseconds the loop may take: well under, quadratic takes seconds
const PIECES: i64 = 20_000;
const MILLISECONDS: i64 = 1500;

#[test]
fn a_loop_over_the_characters_of_a_text_takes_linear_time() {
	let program = format!("t = \"\"; for i in 0..{PIECES} {{ t += \"ab\" }}; t0 = clock(); c = 0; for ch in t {{ if ch == 'a' {{ c += 1 }} }}; \
		if clock() - t0 < {MILLISECONDS} then c else -1");
	is!(&program, PIECES);
	// another text counted in between, and a text grown after its count
	is!("t = \"abc\"; w = \"xy\"; n = 0; for ch in t { n += #w }; t += \"d\"; [n, #t]", warp::parse("[6 4]"));
}

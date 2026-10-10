// card play-mozart: `./` or `../` glued to what follows, at the start of an operand, is a file path written without
// quotes: `play ./mozart.mp3`. With a space behind it `./` stays the element-wise division: `[2 4] ./ 2`
use crate::is;
use warp::{parse, Node};

#[test]
fn a_dot_slash_starts_a_path() {
	assert_eq!(parse("./mozart.mp3"), Node::Text("./mozart.mp3".into()));
	assert_eq!(parse("../music/mozart.mp3"), Node::Text("../music/mozart.mp3".into()));
	is!("x = ./songs/mozart.mp3\nx", "./songs/mozart.mp3");
	is!("[./a.wav, ../b.wav]", parse("[\"./a.wav\" \"../b.wav\"]"));
}

#[test]
fn a_spaced_dot_slash_divides_each() {
	is!("[2 4] ./ 2", parse("[1 2]"));
	is!("xs = [2 4]\nxs./2", parse("[1 2]"));
}

#[cfg(feature = "native")]
#[test]
fn a_path_plays() {
	crate::common::fails_with("play ./no/such/mozart.mp3", "no/such/mozart.mp3");
}

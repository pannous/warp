//! card self-differential: the harness that checks each warp-written compiler part against its Rust twin
//! (tests/common/twins.rs); its first twin is the lexer's columns: the width of each line in characters
use crate::common::twins::{differential, sources};
use warp::{Bracket, Node, Separator};

fn widths_of_lines(text: &str, width: fn(&str) -> usize) -> Node {
	let widths = text.split('\n').map(|line| Node::from(width(line) as i64));
	Node::List(widths.collect(), Bracket::Square, Separator::None)
}

/// What the parser counts a column in (warp_parser/scanning.rs advance): one char
fn line_widths(text: &str) -> Node {
	widths_of_lines(text, |line| line.chars().count())
}

#[test]
fn every_sample_and_probe_is_a_source() {
	let found = sources();
	assert!(found.iter().any(|path| path.ends_with("samples/hello/hello.warp")));
	assert!(found.iter().any(|path| path.starts_with("probes/self_hosting")));
}

#[test]
#[ignore = "fixnow: fixnow-shalom-twins: differs from its Rust twin on probes/shalom_characters.warp (grapheme widths)"]
fn line_widths_match_their_rust_twin() {
	differential("positions", "line_widths", line_widths);
}

/// a twin that counts bytes, not characters, differs on the first source with a non-ASCII line
#[test]
#[should_panic(expected = "line_widths differs from its Rust twin on")]
fn a_wrong_twin_is_reported_with_its_first_difference() {
	differential("positions", "line_widths", |text| widths_of_lines(text, str::len));
}

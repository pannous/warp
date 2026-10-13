//! card self-lexer: the lexer written in warp (lib/compiler/lexer.warp) next to its Rust twins, over every sample and
//! probe (tests/common/twins.rs)
use crate::common::twins::{differential, warp_list};
use warp::warp_parser::comment_and_text_spans;
use warp::Node;

/// [[start, end], …] of the comments and texts, in characters
fn spans(text: &str) -> Node {
	let pair = |(start, end): (usize, usize)| warp_list([start, end].into_iter().map(|at| Node::from(at as i64)));
	warp_list(comment_and_text_spans(text).into_iter().map(pair))
}

#[test]
fn comment_and_text_spans_match_their_rust_twin() {
	differential("lexer", "spans", spans);
}

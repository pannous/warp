//! card self-lexer: the lexer written in warp (lib/compiler/lexer.warp) next to its Rust twins, over every sample and
//! probe (tests/common/twins.rs)
use crate::common::twins::{differential, warp_list};
use warp::warp_parser::{comment_and_text_spans, read_atoms, AtomKind};
use warp::Node;

/// [[start, end], …] of the comments and texts, in characters
fn spans(text: &str) -> Node {
	warp_list(comment_and_text_spans(text).into_iter().map(|(start, end)| pair(start, end)))
}

fn pair(first: usize, second: usize) -> Node {
	warp_list([first, second].into_iter().map(|at| Node::from(at as i64)))
}

/// [[line, column], …] of the atoms of `kind` the parser reads, in source order
fn positions(text: &str, kind: AtomKind) -> Node {
	let mut found: Vec<(usize, usize)> =
		read_atoms(text).into_iter().filter(|atom| atom.kind == kind).map(|atom| (atom.line, atom.column)).collect();
	found.sort();
	warp_list(found.into_iter().map(|(line, column)| pair(line, column)))
}

#[test]
fn numbers_match_their_rust_twin() {
	differential("lexer", "numbers", |text| positions(text, AtomKind::Number));
}

#[test]
fn texts_match_their_rust_twin() {
	differential("lexer", "texts", |text| positions(text, AtomKind::Text));
}

#[test]
fn comment_and_text_spans_match_their_rust_twin() {
	differential("lexer", "spans", spans);
}

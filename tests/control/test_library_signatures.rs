//! Card reflect-library-signatures (notes/reflection.md step 3): library and host words answer `params`, `signature`,
//! `dir` and `help` like a user function: named where the library lists them, else positional from the word's table
use crate::is;
use warp::{texts, Node};

fn text(value: &str) -> Node {
	Node::Text(value.into())
}

#[test]
fn an_operator_words_signature() {
	is!("sqrt.signature", text("(x:number) -> number"));
	is!("abs.params", texts(vec!["x"]));
	is!("dir(sqrt)", texts(vec!["params", "signature"]));
	is!("help(cbrt)", text("cbrt(x:number) -> number"));
}

#[test]
fn a_prelude_words_signature_as_its_head() {
	is!("first.signature", text("(xs)"));
	is!("replace.params", texts(vec!["t", "old", "new"]));
	is!("help(nearest)", text("nearest(x, kind, limit = 1000000, within = 0)"));
}

#[test]
fn table_words_positional() {
	is!("atan2.signature", text("(a:float, b:float) -> float"));
	is!("trim.signature", text("(x) -> text"));
	is!("split.parameters", texts(vec!["t", "separator"]));
	is!("sorted.signature", text("(xs:list) -> list"));
}

#[test]
fn the_programs_own_definition_wins() {
	is!("def sort(a){a}; sort.params", texts(vec!["a"]));
}

//! card holder-says: a named argument holding a lambda, `Holder(d: x => x + 1)`, gives d the lambda as `{d: x => x + 1}`
//! does; before, `:` binding tighter than `=>` made it the lambda `(d: x) => x + 1` and x undefined. A typed lambda
//! `(x: int => x * 2)` stays one
use crate::is;
use warp::warp_parser::WarpParser;
use warp::{Node, Op};

/// The argument after the call's head: `Holder(d: …)` → `d: …`
fn argument(code: &str) -> Node {
	match WarpParser::parse(code).drop_meta() {
		Node::List(items, _, _) => items[1].drop_meta().clone(),
		other => panic!("{code}: no call, {other:?}"),
	}
}

#[test]
fn a_named_argument_holds_the_lambda() {
	let Node::Key(name, Op::Colon, value) = argument("Holder(d: x => x + 1)") else { panic!("no named argument") };
	assert!(name.drop_meta().is_symbol("d"));
	// the parsed program holds `=>` as Op::Arrow (warp_parser/mod.rs)
	assert!(matches!(value.drop_meta(), Node::Key(_, Op::Arrow, _)), "{value:?}");
	let Node::Key(head, Op::Arrow, _) = argument("f(x: int => x * 2)") else { panic!("no lambda") };
	assert!(matches!(head.drop_meta(), Node::Key(_, Op::Colon, _)), "a typed lambda stays: {head:?}");
	is!("apply(f, v) := f(v)\napply(x: int => x * 2, 5)", 10);
}

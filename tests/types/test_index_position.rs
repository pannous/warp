// card index-position (user, 2026-10-12): an index counts from 0 (`xs[i]`), a position from 1 (`xs#p`); index_of
// gives a position, argmax and argmin an index, `insert at:` takes an index, and each converts where it meets the
// other's brackets, so `xs[xs.index_of(20)]` picks 20 (notes/index_position.md)
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

const XS: &str = "xs = [10 20 30]; ";

fn with_xs(code: &str) -> String {
	format!("{XS}{code}")
}

#[test]
fn a_position_meets_brackets() {
	is!(&with_xs("xs[xs.index_of(20)]"), 20);
	is!(&with_xs("p = xs.index_of(30); xs[p]"), 30);
	is!(&with_xs("xs#(xs.index_of(20))"), 20);
	is!(&with_xs("xs.index_of(20)"), 2);
}

#[test]
fn an_index_meets_the_hash() {
	is!(&with_xs("xs#(xs.argmax())"), 30);
	is!(&with_xs("xs[argmax(xs)]"), 30);
	is!(&with_xs("i = argmin(xs); xs#i"), 10);
	is!(&with_xs("argmax(xs)"), 2);
}

#[test]
fn declared_positions_and_indexes() {
	is!(&with_xs("p: position = 2; xs[p]"), 20);
	is!(&with_xs("i: index = 2; xs#i"), 30);
	is!(&with_xs("f(p: position) := xs[p]; f(1)"), 10);
	fails_with(&with_xs("p: position = 2.5; p"), "int");
}

#[test]
fn arithmetic_keeps_the_dimension() {
	// user decision: position ± int stays a position, index ± int an index, as size_t or a pointer does
	is!(&with_xs("xs[xs.index_of(10) + 1]"), 20);
	is!(&with_xs("xs#(xs.index_of(10) + 1)"), 20);
	is!(&with_xs("p = xs.index_of(30); xs[p - 1]"), 20);
	is!(&with_xs("xs#(argmin(xs) + 1)"), 20);
	is!(&with_xs("next = xs.index_of(10) + 1; xs[next]"), 20);
	// position − position is a plain count
	is!(&with_xs("p = xs.index_of(10); q = xs.index_of(30); q - p"), 2);
	is!(&with_xs("p = xs.index_of(10); q = xs.index_of(30); xs[q - p]"), 30);
	fails_with(&with_xs("p = xs.index_of(10); q = xs.index_of(30); p + q"), "position");
}

#[test]
fn insert_at_takes_an_index() {
	assert_eq!(eval(&with_xs("xs.insert(15, at: xs.index_of(20)); xs")).serialize(), "[10 15 20 30]");
	assert_eq!(eval(&with_xs("xs.insert(5, at: 0); xs")).serialize(), "[5 10 20 30]");
}

#[test]
fn a_missing_position_is_out_of_range() {
	fails_with(&with_xs("xs[xs.index_of(99)]"), "index out of range");
}

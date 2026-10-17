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
	is!("atan2.signature", text("(a: float, b: float) -> float"));
	is!("trim.signature", text("(x) -> text"));
	is!("split.parameters", texts(vec!["t", "separator"]));
	is!("sorted.signature", text("(xs:list) -> list"));
}

#[test]
fn the_programs_own_definition_wins() {
	is!("def sort(a){a}; sort.params", texts(vec!["a"]));
}

/// The whole-number words of the math module declare int parameters, so their signatures show them; a word that
/// takes any number keeps parameters of the passed kind, shown bare (card signature-signature)
#[test]
fn whole_number_math_words_show_int_parameters() {
	is!("use math; gcd.signature", text("(a: int, b: int) -> int"));
	is!("use math; factorial.signature", text("(n: int) -> int"));
	is!("use math; gcd(2^70, 2^65) == 2^65", true);
	is!("use math; lerp.params", texts(vec!["a", "b", "t"]));
}

/// A function whose result is a bool says so: `-> bool`, not the Int a bool is held as (card function-returning)
#[test]
fn a_bool_result_shows_as_bool() {
	is!("f(b) := b == 0; (function f).signature", text("(b) -> bool"));
	is!("use math; is_prime.signature", text("(n: int) -> bool"));
	is!("f(b) := b == 0; type(function f)", "function: (b) -> bool");
	is!("f(b) := b + 1; (function f).signature", text("(b) -> int"));
}

/// A function returning another bool function's result, or its own, is a bool function too (card signature-int)
#[test]
fn a_bool_through_calls_shows_as_bool() {
	is!("g(x) := x > 2; h(x) := g(x); (function h).signature", text("(x) -> bool"));
	is!("f(n) := if n == 0 then true else f(n - 1); (function f).signature", text("(n) -> bool"));
	is!("f(x) := x > 2; f.return_type", text("bool"));
	is!("f(x) := x > 2; fs = [function f]; fs#1.return_type", text("bool"));
}

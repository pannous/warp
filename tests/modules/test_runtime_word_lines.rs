//! The words warp computes itself are lib/prelude.warp's `from warp` lines (card retire-library-words): their
//! parameters, defaults and results come from there, and each is a function a program may call
use crate::is;

#[test]
fn a_left_out_parameter_takes_its_default() {
	is!("count(\"a b c\".split)", 3);
	is!("{a: 1}.get(\"b\", 7)", 7);
	is!("\"hello\".slice(2)", "llo");
}

#[test]
fn the_runtime_words_are_callable_by_name() {
	is!("map_without({a: 1, b: 2}, \"a\")", warp::warp_parser::parse("{b:2}"));
	is!("map_of_entry(a: 1)", warp::warp_parser::parse("{a:1}"));
	is!("class P { x: int }; p = P(1); q = instance_copy(p, false); q.x", 1);
}

#[test]
fn a_signature_is_its_line() {
	is!("split.signature", "(t:text, separator:text = \" \") -> list of text");
	is!("reverse.signature", "(xs:list) -> list"); // its first overload (card typed-builtins)
}

//! Reflection words in all their spellings (card reflection-synonyms): `properties` lists the fields like `fields`,
//! `attributes`, `members`; `functions` lists the methods like `methods`; each also as a call `fields(p)`.
//! A real field or a user function of that name wins.
use crate::is;
use warp::texts;

const POINT: &str = "class P{x:int; y:int; sq() := x*x+y*y}; p = P(1, 2)";

#[test]
fn properties_lists_the_fields() {
	is!(&format!("{POINT}; p.properties"), texts(vec!["x", "y"]));
	is!(&format!("{POINT}; properties of p"), texts(vec!["x", "y"]));
	is!(&format!("{POINT}; P(3, 4).properties"), texts(vec!["x", "y"]));
	is!("m = {a:1}; properties(m)", texts(vec!["a"]));
}

#[test]
fn reflection_words_as_calls() {
	is!(&format!("{POINT}; fields(p)"), texts(vec!["x", "y"]));
	is!(&format!("{POINT}; fields(P)"), texts(vec!["x", "y"]));
	is!(&format!("{POINT}; attributes(P(1, 2))"), texts(vec!["x", "y"]));
	is!("m = {a:1}; fields(m)", texts(vec!["a"]));
}

#[test]
fn functions_lists_the_methods() {
	is!(&format!("{POINT}; [m.name for m in functions(p)]"), texts(vec!["sq"]));
	is!(&format!("{POINT}; [m.name for m in functions(P)]"), texts(vec!["sq"]));
	is!(&format!("{POINT}; count(functions of p)"), 1);
}

#[test]
fn a_real_field_named_like_a_synonym_wins() {
	is!("class Q{properties:int}; Q(3).properties", 3);
	is!("class Q{properties:int}; q = Q(3); q.properties", 3);
	is!("class Q{members:int}; Q(3).members", 3);
}

#[test]
fn a_user_function_named_like_a_synonym_wins() {
	is!("fields(x) := 42; fields(1)", 42);
}

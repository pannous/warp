//! D5 (user, 2026-10-03): matching by type name, "General rule" (wiki/matching.md, type.md; rule in notes/matching.md).
//! Known type words in a definition head name typed parameters.

use warp::is;
use warp::wasm_emitter::eval;

#[test]
fn test_type_then_name_parameter() {
	is!("fib int i = if i<2 : i else fib(i - 1) + fib(i - 2); fib(10)", 55);
	is!("twice int x := x+x; twice 4", 8);
}

#[test]
fn test_type_word_is_the_parameter_name() {
	is!("fibonacci number = if number<2 : number else fibonacci(number - 1) + fibonacci(number - 2); fibonacci 10", 55);
	is!("succ int := int+1; succ 3", 4);
}

#[test]
fn test_lone_type_word_parameter_is_it() {
	is!("foo of int = it + it; foo 3", 6);
	is!("square of a number = it*it; square 3", 9);
}

#[test]
fn test_to_phrase_with_a_type_word() {
	is!("to square a number: it*it; square 3", 9);
	is!("to cube a number: number*number*number; cube 2", 8);
	is!("to halve number x: x/2; halve 3", eval("1.5"));
}

// The general rule (notes/matching.md), approved by warp-43 as an assumption for the user to review

#[test]
fn test_prepositions_separate_parameter_slots() {
	is!("to add number a to number b: a+b; add(1, 2)", 3);
	is!("to add a to b: a+b; add(1, 2)", 3);
	is!("to add a b: a+b; add 2 3", 5);
}

#[test]
fn test_an_unknown_noun_is_an_untyped_name() {
	is!("to measure a parcel: parcel+1; measure 2", 3);
}

#[test]
fn test_a_declared_class_types_its_noun() {
	let definition = warp::wasp_parser::parse("class photo{width:int}; to keep a photo: photo").serialize();
	assert!(definition.contains("keep photo:photo"), "{definition}");
	let untyped = warp::wasp_parser::parse("to keep a photo: photo").serialize();
	assert!(!untyped.contains("photo:photo"), "{untyped}");
}

#[test]
fn test_phrases_differing_only_by_unknown_nouns_are_a_redefinition() {
	crate::common::fails_with("to kill a person: 1; to kill a dog: 2; kill 0", "kill is defined twice; declare class person and class dog");
	is!("class person{name}; class dog{name}; to kill a person: 1; to kill a dog: 2; 3", 3);
}

#[test]
fn test_a_multi_word_noun_is_named_and_typed_by_its_head() {
	is!("to call a phone number: number+1; call 41", 42);
	crate::common::fails_with("to join a first name with a last name: name; 1", "use one-word parameter names");
}

//! Traits (interfaces, protocols, typeclasses): Comparable and Equatable, built in for the existing kinds and adopted by a user
//! type that defines the operation (notes/traits.md). `sort`, `min`/`max`, `<` and `==` dispatch through them.

use crate::common::fails_with;
use warp::is;

const PERSON: &str = "class person{name age:int}; compare(a:person, b:person) := a.age - b.age; ";
const DOT: &str = "class dot{x:int}; ";

fn with(prelude: &str, code: &str) -> String {
	format!("{prelude}{code}")
}

#[test]
fn test_builtin_kinds_stay_comparable() {
	is!("(sort [\"b\" \"a\"])#1", "a");
	is!("(sort [3 1 2])#1", 1);
	is!("(sort [2.5 1 3])#2", 2.5);
}

#[test]
fn test_a_user_type_sorts_through_its_compare() {
	is!(&with(PERSON, "xs = [person(\"Ann\" 40) person(\"Bob\" 30)]; first = (sort xs)#1; first.name"), "Bob");
	is!(&with(PERSON, "first = (sort [person(\"Ann\" 40) person(\"Bob\" 30) person(\"Cy\" 35)])#2; first.name"), "Cy");
}

#[test]
fn test_a_user_type_orders_with_less_than() {
	is!(&with(PERSON, "p = person(\"Ann\" 40); q = person(\"Bob\" 30); q < p"), true);
	is!(&with(PERSON, "p = person(\"Ann\" 40); q = person(\"Bob\" 30); p <= q"), false);
}

#[test]
fn test_min_and_max_of_a_user_type() {
	is!(&with(PERSON, "young = min(person(\"Ann\" 40), person(\"Bob\" 30)); young.name"), "Bob");
	is!(&with(PERSON, "xs = [person(\"Ann\" 40) person(\"Bob\" 30)]; old = max(xs); old.name"), "Ann");
}

#[test]
fn test_two_user_types_each_have_their_own_compare() {
	let code = "class box{size:int}; compare(a:box, b:box) := a.size - b.size; first = (sort [box(3) box(1)])#1; first.size";
	is!(&with(PERSON, code), 1);
	let both = "class box{size:int}; compare(a:box, b:box) := b.size - a.size; p = (sort [person(\"A\" 2) person(\"B\" 1)])#1; b = (sort [box(1) box(3)])#1; p.age * 10 + b.size";
	is!(&with(PERSON, both), 13);
}

#[test]
fn test_sorting_a_type_without_compare_names_the_trait_and_the_fix() {
	fails_with(&with(DOT, "sort [dot(2) dot(1)]"), "dot is not Comparable");
	fails_with(&with(DOT, "sort [dot(2) dot(1)]"), "compare(a:dot, b:dot)");
	fails_with(&with(DOT, "xs = [dot(2) dot(1)]; sort xs"), "dot is not Comparable");
}

#[test]
fn test_ordering_a_type_without_compare_is_a_compile_error() {
	fails_with(&with(DOT, "dot(1) < dot(2)"), "dot is not Comparable");
	fails_with(&with(DOT, "max(dot(1), dot(2))"), "dot is not Comparable");
}

#[test]
fn test_mixed_kinds_stay_not_comparable() {
	fails_with("sort [1 \"a\"]", "not comparable");
}

#[test]
fn test_conformance_is_a_type_test() {
	is!("3 is Comparable", true);
	is!("\"a\" is Comparable", true);
	is!("[1 2] is Comparable", false);
	is!(&with(PERSON, "person(\"Ann\" 40) is Comparable"), true);
	is!(&with(DOT, "dot(1) is Comparable"), false);
	is!(&with(DOT, "dot(1) is Equatable"), true);
}

#[test]
fn test_instances_are_equatable_by_their_fields() {
	is!(&with(DOT, "dot(1) == dot(1)"), true);
	is!(&with(DOT, "dot(1) == dot(2)"), false);
}

#[test]
fn test_a_user_type_overrides_equality_with_equals() {
	let word = "class word{spelling}; equals(a:word, b:word) := a.spelling.lower == b.spelling.lower; ";
	is!(&with(word, "word(\"Hi\") == word(\"hi\")"), true);
	is!(&with(word, "word(\"Hi\") != word(\"ho\")"), true);
}

#[test]
fn test_a_compare_with_mismatched_types_is_no_conformance() {
	fails_with("class dot{x:int}; compare(a:dot, b:int) := a.x - b; sort [dot(2) dot(1)]", "compare(a:dot, b:dot)");
}

#[test]
fn test_an_explicit_conformance_claim_needs_the_operation() {
	fails_with("class dot{x:int} is Comparable; 1", "dot claims Comparable");
	is!("class dot{x:int} is Comparable; compare(a:dot, b:dot) := a.x - b.x; first = (sort [dot(2) dot(1)])#1; first.x", 1);
}

const SHAPES: &str = "trait shape{area}; class square{side:int}; class rect{w:int h:int}; area(s:square) := s.side*s.side; area(r:rect) := r.w*r.h; ";

#[test]
fn test_a_declared_trait_dispatches_by_type() {
	is!(&with(SHAPES, "area(square(3)) + area(rect(2 5))"), 19);
	is!(&with(SHAPES, "r = rect(2 5); area(r)"), 10);
	is!(&with(SHAPES, "total(s:square) := area(s) + 1; total(square(2))"), 5);
}

#[test]
fn test_conformance_to_a_declared_trait_is_a_type_test() {
	is!(&with(SHAPES, "square(1) is shape"), true);
	is!(&with(SHAPES, "class dot{x:int}; dot(1) is shape"), false);
}

#[test]
fn test_a_type_without_the_operation_of_a_declared_trait() {
	fails_with("trait shape{area}; class dot{x:int}; area(dot(1))", "dot is not shape");
	fails_with("trait shape{area}; class dot{x:int}; area(dot(1))", "define area(x:dot)");
	fails_with("trait shape{area(s)}; class dot{x:int}; area(dot(1))", "define area(s:dot)");
}

#[test]
fn test_a_claim_of_a_declared_trait_is_checked() {
	fails_with("trait shape{area}; class dot{x:int} is shape; 1", "dot claims shape");
	is!("trait shape{area}; class dot{x:int} is shape; area(d:dot) := d.x; area(dot(4))", 4);
}

#[test]
fn test_trait_synonyms_declare_a_trait() {
	for keyword in ["trait", "interface", "protocol", "typeclass", "prototype", "capability", "aspect", "feature"] {
		let code = format!("{keyword} shape{{area}}; class square{{side:int}}; area(s:square) := s.side*s.side; area(square(3))");
		is!(&code, 9);
	}
}

#[test]
fn test_a_builtin_trait_cannot_be_redeclared() {
	fails_with("trait Comparable{compare}; 1", "Comparable is a built-in trait");
}

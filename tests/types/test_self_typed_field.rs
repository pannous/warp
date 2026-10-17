//! card empty-field (user decision (b), strict): a field of the class's own type without `?` could never be filled,
//! every Tree needing a Tree first, so it is a compile error at the class; `left: Tree?` holds ø or a Tree
use crate::common::fails_with;
use crate::is;

#[test]
fn a_required_field_of_the_own_class_is_an_error() {
	fails_with("class Tree{left: Tree; v:int}; Tree(ø, 3).v", "left can never be filled");
	fails_with("class Tree{left: Tree; v:int}; 1", "fix: left: Tree?");
}

#[test]
fn an_optional_field_of_the_own_class_holds_nothing() {
	is!("class Tree{left: Tree?; v:int}; Tree(ø, 3).v", 3);
	is!("class Tree{kids: [Tree]; v:int}; Tree([], 4).v", 4);
}

#[test]
fn other_fields_stay_strict() {
	fails_with("class P{n: int}; P(ø).n", "not an int");
}

//! P215: lists are shared, so a view of a declared list under a wider element type that changes it would put items of
//! another type into the list (TypeScript's array covariance hole, probes/variance/). Where the alias is visible, a
//! parameter or a declared variable of the wider type that changes the list, it is a compile error; a view that only
//! reads it, or a narrower one, is fine (card p215-user)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_changed_wider_view_of_a_declared_list_is_a_compile_error() {
	fails_with("names: texts = [\"hi\"]; widen(xs: list) := { xs.add(420); xs }; widen(names); names", "widen(xs: list) changes names, which is declared texts");
	fails_with("names: list of text = [\"hi\"]; widen(xs: list of (text or number)) := { xs.add(420); xs }; widen(names); names", "changes names, which is declared list of text");
	fails_with("names: texts = [\"hi\"]; ys: list = names; ys.add(420); names", "ys: list changes names, which is declared texts");
	fails_with("names: texts = [\"hi\"]; widen(xs: list) := { xs#1 = 420; xs }; widen(names); names", "changes names");
	fails_with("class bag { items: texts }; b = bag([\"hi\"]); widen(xs: list) := { xs.add(420); xs }; widen(b.items); b.items", "changes b.items, which is declared texts");
}

const SHAPES: &str = "class Shape { a: int }; class Circle extends Shape { r: int }; c = Circle(1, 2); ";

#[test]
fn a_changed_parent_class_view_of_a_subclass_list_is_a_compile_error() {
	fails_with(&format!("{SHAPES}xs: [Circle] = [c]; ys: [Shape] = xs; ys.add(Shape(3)); xs"), "ys: [Shape] changes xs, which is declared [Circle]");
	fails_with(&format!("{SHAPES}xs: [Circle] = [c]; f(ys: [Shape]) := {{ ys.add(Shape(3)); ys }}; f(xs); xs"), "f(ys: [Shape]) changes xs, which is declared [Circle]");
	is!(&format!("{SHAPES}xs: [Circle] = [c]; ys: [Shape] = xs; count ys"), 1);
	is!(&format!("{SHAPES}xs: [Circle] = [c]; f(ys: [Shape]) := count ys; f(xs)"), 1);
}

/// Lists of instances declared `[Circle]` (cards class-list-add, class-list-param: they were held as an Int)
#[test]
fn a_bracketed_class_list_is_a_list() {
	is!(&format!("{SHAPES}xs: [Circle] = [c]; xs.add(Circle(5, 6)); xs#2.r"), 6);
	is!(&format!("{SHAPES}ys: [Shape] = [c]; ys.add(Shape(3)); count ys"), 2);
	is!(&format!("{SHAPES}xs: [Circle] = [c]; f(ys: [Circle]) := count ys; f(xs)"), 1);
}

#[test]
fn a_reading_or_narrower_view_is_fine() {
	is!("names: texts = [\"hi\"]; size(xs: list) := count xs; size(names)", 1);
	is!("names: texts = [\"hi\"]; ys: list = names; count ys", 1);
	is!("names: texts = [\"hi\"]; more(ys: texts) := { ys.add(\"ho\"); ys }; more(names); count names", 2);
}

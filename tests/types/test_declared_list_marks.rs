//! P215 run-time half: where the alias of a declared list is not visible (a lax variable, an unannotated parameter), the
//! list itself carries its element type (an element mark on its head cell), and each write through any alias checks the
//! item: a loud run-time error, a fitting item stays fine (card p215-user)
use crate::common::fails_with;
use crate::is;

const CANNOT_HOLD: &str = "list cannot hold this item";

#[test]
fn a_write_of_another_type_through_a_hidden_alias_fails_at_run_time() {
	fails_with("names: texts = [\"hi\"]; f(xs) := xs.add(420); f(names); names", CANNOT_HOLD);
	fails_with("names: texts = [\"hi\"]; ys = names; ys.add(420); names", CANNOT_HOLD);
	fails_with("names: texts = [\"hi\"]; ys = names; ys#1 = 3; names", CANNOT_HOLD);
	fails_with("names: texts = [\"hi\"]; ys = names; ys.insert(5, at: 0); names", CANNOT_HOLD);
	fails_with("xs: ints = [1]; f(ys) := ys.add(\"a\"); f(xs); xs", CANNOT_HOLD);
	fails_with("xs: bools = [yes]; f(ys) := ys.add(7); f(xs); xs", CANNOT_HOLD);
	fails_with("names: [text] = [\"hi\"]; ys = names; ys.add(1); names", CANNOT_HOLD);
	is!("names: texts = [\"hi\"]; ys = names; try { ys.add(1) } catch e { \"caught\" }", "caught");
}

#[test]
fn a_fitting_write_through_a_hidden_alias_is_fine() {
	is!("names: texts = [\"hi\"]; f(xs) := xs.add(\"yo\"); f(names); count names", 2);
	is!("xs: ints = [1]; f(ys) := ys.add(2); f(xs); xs#2", 2);
	is!("xs: floats = [1.5]; f(ys) := ys.add(2); f(xs); count xs", 2);
	is!("xs: bools = [yes]; f(ys) := ys.add(no); f(xs); count xs", 2);
}

/// The mark sits above the bracket byte: comparing, printing and the type see the list as before
#[test]
fn a_marked_list_is_the_same_list() {
	is!("names: texts = [\"hi\"]; ys = names; names == [\"hi\"]", true);
	is!("names: texts = [\"hi\"]; ys = names; ys.add(\"a\"); string(names)", r#"["hi" "a"]"#);
	is!("names: texts = [\"hi\"]; ys = names; ys.add(\"a\"); type(names)", "list of text");
}

/// An empty declared list (ø) carries its mark too, and keeps it when its first item makes it a cell (card p215-empty)
#[test]
fn an_empty_declared_list_checks_its_items() {
	fails_with("names: texts = []; ys = names; ys.add(1); names", CANNOT_HOLD);
	fails_with("names: texts = []; f(xs) := xs.add(420); f(names); names", CANNOT_HOLD);
	fails_with("names: texts = []; ys = names; ys.add(\"a\"); ys.add(2); names", CANNOT_HOLD);
	fails_with("names: texts = []; ys = names; ys.insert(5, at: 0); names", CANNOT_HOLD);
	fails_with("names: texts = [\"a\"]; ys = names; ys.pop(); ys.add(3); names", CANNOT_HOLD);
	is!("names: texts = []; ys = names; ys.add(\"a\"); string(names)", r#"["a"]"#);
	is!("names: texts = []; ys = names; count names", 0);
	is!("names: texts = []; ys = names; names == []", true);
	is!("names: texts = []; ys = names; if names { 1 } else { 2 }", 2);
}

/// Only a type written in the source marks a list: the element type inferred for `ys = [1]` leaves it open
#[test]
fn an_inferred_element_type_marks_nothing() {
	is!("ys = [1]; zs = ys; zs.add(\"a\"); count ys", 2);
	is!("ys = [1]; f(xs) := xs.add(\"a\"); f(ys); count ys", 2);
	is!("ys = []; ys.add(1); f(xs) := xs.add(\"a\"); f(ys); count ys", 2);
}

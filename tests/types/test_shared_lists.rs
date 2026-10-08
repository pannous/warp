//! Lists are references (P200b, card shared-lists): every name holding a list sees an item added, removed or set
//! through another, as in Python or JavaScript. `+` makes a new list; `copy()` makes an independent one.
use crate::is;

#[test]
fn an_alias_sees_an_added_item() {
	is!("xs = [1]; ys = xs; ys.add(2); count(xs)", 2);
	is!("xs = [1, 2]; ys = xs; ys.add(3); xs#3", 3);
	is!("xs = [1, 2]; ys = xs; xs.add(3); count(ys)", 3);
}

#[test]
fn an_alias_sees_a_set_item() {
	is!("xs = [1, 2]; ys = xs; ys#1 = 5; xs#1", 5);
	is!("xs = [\"a\", \"b\"]; ys = xs; ys#2 = \"c\"; xs#2", "c");
}

#[test]
fn an_alias_sees_a_popped_item() {
	is!("xs = [1, 2, 3]; ys = xs; ys.pop(); count(xs)", 2);
}

#[test]
fn a_function_changes_the_list_it_is_given() {
	is!("f(l) := l.add(3); xs = [1, 2]; f(xs); count(xs)", 3);
	is!("f(l) := l#1 = 9; xs = [1, 2]; f(xs); xs#1", 9);
}

#[test]
fn a_list_in_a_list_is_shared() {
	is!("xs = [1]; outer = [xs]; xs.add(2); count(outer#1)", 2);
}

#[test]
fn plus_makes_a_new_list() {
	is!("xs = [1]; zs = xs + [2]; xs.add(9); count(zs)", 2);
	is!("xs = [1]; zs = [0] + xs; xs.add(9); count(zs)", 2);
	is!("xs = [1]; ys = xs; ys = ys + [2]; count(xs)", 1);
}

#[test]
fn copy_makes_an_independent_list() {
	is!("xs = [1]; ys = xs.copy(); ys.add(2); count(xs)", 1);
	is!("xs = [1, 2]; ys = xs.copy(); ys#1 = 5; xs#1", 1);
}

#[test]
fn each_literal_is_a_new_list() {
	is!("f() := { xs = []; xs.add(1); count(xs) }; f() + f()", 2);
	is!("s = 0; for i in [1, 2, 3] { xs = [0]; xs.add(i); s += count(xs) }; s", 6);
}

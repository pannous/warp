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

#[test]
fn an_alias_sees_a_removed_item() {
	is!("xs = [1, 2, 3]; ys = xs; ys.remove(1); count(xs)", 2);
	is!("xs = [1, 2, 3]; ys = xs; ys.remove(2); xs#2", 3);
	is!("xs = [1, 2, 3]; ys = xs; ys.remove(3); count(xs)", 2);
	is!("xs = [1]; ys = xs; ys.remove(1); count(xs)", 0);
	is!("xs = [1, 2, 1]; xs.remove(1); xs", warp::ints(vec![2, 1]));
}

#[test]
fn an_alias_sees_a_removed_entry() {
	is!("m = {a: 1, b: 2}; n = m; n.remove(\"a\"); count(m)", 1);
	is!("m = {a: 1, b: 2}; n = m; n.remove(\"b\"); count(m)", 1);
	is!("m = {a: 1, b: 2}; m.remove(\"a\")", 1);
}

#[test]
fn an_alias_sees_an_inserted_item() {
	is!("xs = [\"a\", \"c\"]; ys = xs; ys.insert(1, \"b\"); xs#2", "b");
	is!("xs = [\"a\"]; ys = xs; ys.insert(0, \"z\"); xs#1", "z");
	is!("xs = [\"a\"]; ys = xs; ys.insert(-1, \"q\"); count(xs)", 2);
}

#[test]
fn a_list_held_by_a_field_or_item_grows_in_place() {
	is!("class Bag { items: list }; b = Bag([1]); xs = b.items; b.items.add(2); count(xs)", 2);
	is!("class Holder{items:[int]}; p = Holder([3]); q = p.items; p.items.add(9); count(q)", 2);
	is!("class Bag { items: list }; b = Bag([]); b.items.add(2); count(b.items)", 1);
	is!("p = {xs: [1]}; ys = p.xs; p.xs.add(5); count(ys)", 2);
	is!("m = [[1], [2]]; row = m#2; m#2.add(5); count(row)", 2);
}

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

#[test]
fn a_typed_alias_of_a_field_or_item_is_no_snapshot() {
	// card shared-lists-typed: the array copy of `ys = p.xs` stays only where no list is changed in place after it
	is!("p = {xs: [1, 2]}; ys = p.xs; p.xs#1 = 7; ys#1", 7);
	is!("m = [[1, 2], [3, 4]]; row = m#2; m#2#1 = 9; row#1", 9);
	is!("m = [[1, 2], [3, 4]]; row = m#2; m#2.add(5); count(row)", 3);
	is!("m = [[1, 2], [3, 4]]; s = 0; for i in 1 to 2 { row = m#i; for j in 1 to 2 { s += row#j } }; s", 10);
}

#[test]
fn a_field_grown_by_plus_assign_is_seen_by_its_holders() {
	// card speed-parallel-emit: `p.items += more` grows the field's list in place, as `xs += more` and the pure lane do
	is!("class Bag { items: list }; b = Bag([1]); xs = b.items; b.items += [2, 3]; count(xs)", 3);
	is!("class Bag { items: any }; b = Bag([1]); xs = b.items; b.items += [2]; count(xs)", 2);
	is!("p = {xs: [1]}; ys = p.xs; p.xs += [5]; count(ys)", 2);
	is!("class Bag { items: list }; b = Bag([]); b.items += [7]; b.items#1", 7);
	is!("class Bag { total: int }; b = Bag(1); b.total += 4; b.total", 5);
	is!("class Tag { name: text }; t = Tag(\"a\"); t.name += \"b\"; t.name", "ab");
	// `+` still makes a new list
	is!("class Bag { items: list }; b = Bag([1]); xs = b.items; b.items = b.items + [2]; count(xs)", 1);
}

#[test]
fn a_field_grown_step_by_step_takes_linear_steps() {
	// 3000 steps of two items: copying the field's list at each step took ~10^8 steps
	let gathered = "class Bag { items: list }; b = Bag([]); for i in 1 to 3000 { b.items += [i, i] }; count(b.items)";
	warp::util::with_fuel(30_000_000, || is!(gathered, 6000));
}

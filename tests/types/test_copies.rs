//! `copy()` of any object (P205/P207): deep by default, every instance, map, list and entry inside is new and each part
//! held twice (a cycle too) is copied once; `copy(shallow: true)` makes only the object's own entries new. A class's
//! own `copy` wins, and `clone` always calls it.
use crate::is;

#[test]
fn a_copy_is_independent() {
	is!("m = {a:1}; n = m.copy(); n.a = 2; m.a", 1);
	is!("m = {a:1, b:2}; n = m.copy(); n.c = 3; try m.c catch 0", 0);
	is!("class P { x: int }; p = P(1); q = p.copy(); q.x = 2; p.x", 1);
	is!("class P { x: int }; p = P(1); q = p.clone(); q.x = 2; q.x", 2);
}

#[test]
fn a_copy_is_deep() {
	is!("m = {a: {b: 1}}; n = m.copy(); n.a.b = 2; m.a.b", 1);
	is!("class P { x: int }; p = P(1); ps = [p]; qs = ps.copy(); qs#1.x = 9; p.x", 1);
}

#[test]
fn a_copy_copies_a_cycle_once() {
	is!("m = {a:1}; m.me = m; n = m.copy(); n.me.a = 7; m.a + n.a", 8);
}

#[test]
fn a_shallow_copy_shares_the_values() {
	is!("m = {a: {b: 1}}; n = m.copy(shallow: true); n.a.b = 2; m.a.b", 2);
	is!("m = {a: {b: 1}}; n = m.copy(shallow: true); n.a = 5; m.a.b", 1);
}

#[test]
fn a_class_copy_wins_and_clone_calls_it() {
	is!("class P { x: int; copy() := P(x + 100) }; p = P(1); p.copy().x", 101);
	is!("class P { x: int; copy() := P(x + 100) }; p = P(1); p.clone().x", 101);
}

#[test]
fn kotlin_copy_changes_fields() {
	is!("class P { x: int; y: int }; p = P(1, 2); q = p.copy(y = 5); q.y * 10 + p.y", 52);
}

#[test]
fn a_number_copy_is_itself() {
	is!("x = 3; x.copy()", 3);
}

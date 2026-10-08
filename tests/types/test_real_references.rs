//! Class instances are references (P200, card real-references): every name and list item holding an instance sees a
//! change of its fields, as in Python or JavaScript. A construction makes a new instance each time it runs. Maps share
//! too, and a new field grows the same object (P200b); `copy()` makes an independent one.
use crate::is;

const POINT: &str = "class Point { x: int }; ";

#[test]
fn an_alias_sees_a_changed_field() {
	is!(&format!("{POINT}p = Point(1); q = p; q.x = 7; p.x"), 7);
	is!(&format!("{POINT}p = Point(1); q = p; q.x += 2; p.x"), 3);
	is!(&format!("{POINT}p = Point(1); q = p; p.x = 5; q.x"), 5);
}

#[test]
fn a_list_item_is_the_instance_itself() {
	is!(&format!("{POINT}ps = [Point(1)]; p = ps#1; p.x = 7; ps#1.x"), 7);
	is!(&format!("{POINT}p = Point(1); ps = [p]; ps#1.x = 7; p.x"), 7);
	is!(&format!("{POINT}ps = [Point(1), Point(2)]; for p in ps {{ p.x += 10 }}; ps#2.x"), 12);
}

#[test]
fn a_function_changes_the_instance_it_is_given() {
	is!(&format!("{POINT}f(q: Point) := q.x = 7; p = Point(1); f(p); p.x"), 7);
	is!(&format!("{POINT}f(q) := {{ r = q; r.x = 9 }}; p = Point(1); f(p); p.x"), 9);
	is!("class C { n: int; inc() := n += 1 }; k = C(0); j = k; j.inc(); k.n", 1);
}

#[test]
fn each_construction_is_a_new_instance() {
	is!(&format!("{POINT}f() := {{ p = Point(0); p.x += 1; p.x }}; f() + f()"), 2);
	is!(&format!("{POINT}s = 0; for i in [1, 2, 3] {{ p = Point(0); p.x += i; s += p.x }}; s"), 6);
	is!(&format!("{POINT}a = Point(1); b = Point(1); a.x = 5; b.x"), 1);
}

#[test]
fn a_new_field_grows_the_same_instance() {
	is!("class Point { x: int; y: int }; p = Point(1, 2); q = p; q.color = \"red\"; p.color", "red");
	is!(&format!("{POINT}p = Point(1); ps = [p]; p.tag = 3; ps#1.tag"), 3);
	is!(&format!("{POINT}f(q) := q.tag = 4; p = Point(1); f(p); p.tag"), 4);
}

#[test]
fn maps_share() {
	is!("m = {a:1}; n = m; n.a = 2; m.a", 2);
	is!("m = {a:1, b:2}; n = m; n.b = 3; m.b", 3);
	is!("m = {a:1}; n = m; n.c = 3; m.c", 3);
	is!("m = {a:1, b:2}; n = m; n.c = 3; m.c", 3);
	is!("m = {a:1}; f(q) := q.a = 5; f(m); m.a", 5);
	is!("m = {a:1}; ms = [m]; m.a = 6; ms#1.a", 6);
}

#[test]
fn a_map_built_by_its_entries_shares() {
	is!("m = {}; m[\"a\"] = 1; n = m; n[\"a\"] = 2; m[\"a\"]", 2);
	is!("m = {}; m[\"a\"] = 1; n = m; n[\"b\"] = 2; m[\"b\"]", 2);
}

#[test]
fn copy_makes_an_independent_map() {
	is!("m = {a:1}; n = m.copy(); n.a = 2; m.a", 1);
	is!("m = {a:1, b:2}; n = m.copy(); n.c = 3; m", "{a:1 b:2}");
}

//! `{a: 1, s := clock()}`: until P71 is decided, `s := e` in an object is a value entry like `s = e` (evaluated now);
//! `f := it*2` stays a function entry
use warp::is;

#[test]
fn test_define_entries_are_values() {
	is!("o = {a: 1, s := 2+3}; o.s", 5);
	is!("o = {a: 1, s := 2+3}; o.a", 1);
	is!("o = {s := 5}; o.s", 5);
	is!("o = {a := 1, s := 5}; o.a + o.s", 6);
	is!("o = {a: 1, s := clock()}; o.s > 0", 1);
}

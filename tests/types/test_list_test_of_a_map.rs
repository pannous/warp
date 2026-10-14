// Card inside-function: a map held as a value of unknown type (a parameter) is no list at run time, as `m is list` of a
// map variable says
use crate::is;

#[test]
fn a_map_argument_is_no_list() {
	is!("kind_of(x) := if x is list then 1 else 0; kind_of({a:1 b:2})", 0);
	is!("kind_of(x) := if x is list then 1 else 0; kind_of([1 2])", 1);
	is!("kind_of(x) := if x is map then 1 else 0; kind_of({a:1 b:2}) * 10 + kind_of([1])", 10);
}

// a map of one entry is held as its lone key at run time (a block of one pair is that pair, lib/markup.warp's
// attributes rely on it)
#[test]
fn a_one_entry_map_argument_is_a_map() {
	is!("kind_of(x) := if x is map then 1 else 0; kind_of({a:1})", 1);
	is!("type_of(x) := type(x); type_of({a:1})", "map");
	is!("m = {a:1}; m.pop(\"a\"); count(m)", 0);
}

// card single-entry (user): `{a:1}` stays its pair (`{a:1} == (a:1)`) and becomes the map of its one entry where a map
// is needed: a map parameter, a map method changing it, an entry appended into a map
#[test]
fn a_one_entry_map_is_upgraded_where_a_map_is_needed() {
	is!("{a:1} == (a:1)", true);
	is!("f(m:map) := count(m); f({a:1})", 1);
	is!("f(m:map) := type(m); f({a:1})", "map");
	is!("m = {a:1}; m.add({b:2}); count(m)", 2);
	is!("m = {a:1}; m += {b:2}; count(m)", 2);
	is!("m = {a:1, b:2}; m += {c:3}; count(m)", 3);
	is!("f(x) := count(x); m = {a:1}; f(m)", 1);
	is!("f(x) := x is pair; f((a:1))", true);
}

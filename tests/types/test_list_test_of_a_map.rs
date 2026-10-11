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
#[ignore = "next"]
fn a_one_entry_map_argument_is_a_map() {
	is!("kind_of(x) := if x is map then 1 else 0; kind_of({a:1})", 1);
	is!("type_of(x) := type(x); type_of({a:1})", "map");
	is!("m = {a:1}; m.pop(\"a\"); count(m)", 0);
}

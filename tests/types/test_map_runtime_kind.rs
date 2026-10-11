// card map-runtime (from warp-types): a map passed into a function is a map at run time too, not the list it is built
// as (a list whose kind carries the curly bracket); `x is map` tests it. Not yet: a one-entry map `{a: 1}` is a Key at
// run time and reads as key
use crate::is;

#[test]
fn a_map_parameter_is_a_map() {
	is!("f(x) := type(x)\nstr(f({a: 1, b: 2}))", "map");
	is!("f(x) := type(x)\nstr(f([1 2]))", "list");
	is!("f(x) := x is map\nf({a: 1, b: 2})", true);
	is!("f(x) := x is map\nf([1 2])", false);
}

#[test]
fn map_is_a_type_word() {
	is!("m = {a: 1, b: 2}\nm is map", true);
	is!("xs = [1 2]\nxs is map", false);
}

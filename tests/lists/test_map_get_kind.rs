//! `m.get(k)` is worth what `m[k]` is: a list value concatenates, a number adds
use crate::is;
use warp::parse;

#[test]
fn get_gives_the_value_kind_of_the_map() {
	is!("m = {}; m[\"a\"] = [1]; m.get(\"a\") + [2]", parse("[1 2]"));
	is!("m = {a: [1]}; m.get(\"a\") + [2]", parse("[1 2]"));
	is!("m = {a: 1}; m.get(\"a\") + 1", 2);
}

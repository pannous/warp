// Card map-answers: `m is map` answers as type(m) names it, a map or a map of its value type
use crate::is;

#[test]
fn a_map_is_a_map() {
	is!("m = {n: 1, k: 2}; m is map", true);
	is!("m = {n: 1, k: \"x\"}; m is map", true);
	is!("m = {n: 1, k: 2}; m is map of int", true);
	is!("m = {n: 1, k: 2}; m is map of text", false);
	is!("m = [1, 2]; m is map", false);
	is!("m = {n: 1, k: 2}; m is list", false);
}

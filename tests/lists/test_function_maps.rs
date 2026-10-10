// A function's `out = {}` makes only its own `out` a map that number keys key (card map-locals): another function's
// list `out` keeps its positions and its membership, and a map of main stays one inside the functions reading it
use crate::is;

#[test]
fn a_map_local_stays_in_its_function() {
	is!("tally(xs) := { out = {}; for x in xs { out[x] = 1 }; out }
uniq(xs) := { out = []; for x in xs { if not (x in out) { out = out + [x] } }; out }
uniq([1,2,1])", warp::ints(vec![1, 2]));
	is!("include list; unique([1,2,1])", warp::ints(vec![1, 2]));
}

#[test]
fn a_map_of_main_is_one_in_a_function() {
	is!("global d = {}; put(k) := { d[k] = 1 }; put(7); 7 in d", true);
}

#[test]
fn a_parameter_is_not_main_s_map() {
	is!("out = {}; out[1] = 2; second(out) := out[1]; second([5, 6])", 6);
}

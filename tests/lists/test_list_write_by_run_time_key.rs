// card self-asserted: xs[k] = v on a list with a key known only at run time; the self-hosted emitter refused it
// ("a write by a key known only at run time"), which broke `warp rebuild self` once [] stayed the empty list
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_list_takes_a_write_by_a_run_time_key_in_place() {
	is!("f(k) := { xs = [1, 2, 3]; xs[k] = 9; xs }; f(1)", parse("[1 9 3]"));
	is!("f(k) := { xs = []; xs += 4; xs[k] = 5; xs }; f(0)", parse("[5]"));
}

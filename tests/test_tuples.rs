//! A comma list in round brackets is a tuple, whatever its items compute: `(h + 1, 2)` keeps both
use warp::*;

#[test] // samples/snake.wasp: `return (head#1 + dx, head#2 + dy)` returned only the last coordinate
fn test_tuple_of_computed_items() {
	is!("h=1; t=(h + 1, 2); t#1 * 10 + t#2", 22);
	is!("def f(h) { return (h + 1, 2) }; t=f(1); t#1 * 10 + t#2", 22);
	is!("h=(1,2); t=(h#1 + 1, h#2 * 3); count(t)", 2);
}

#[test] // samples/snake.wasp: a loop over a list holding a global tuple, assigning it back to the global
fn test_loop_over_list_with_global_tuple() {
	is!("global d = (1, 0); def f() { for o in [d, (0, 1)] { if o#1 == 0 { d = o; return 1 } }; return 0 }; f(); d#2", 1);
	is!("global d = (1, 0); def f() { for o in [d, (0,1)] { d = o }; return d }; t=f(); t#2", 1);
}

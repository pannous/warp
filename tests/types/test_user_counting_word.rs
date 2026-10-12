//! A user function named like a counting word (`def size(m)`) is the user's function: the map it gets is the caller's
//! one map (P200b), so its changes stay (card user-defined-size)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn test_user_size_changes_the_callers_map() {
	is!("m = {}; m[\"a\"] = 1; def size(m) { m.z = 2; 0 }; size(m); m.z", 2);
	is!("m = {}; m[\"a\"] = 1; def size(m) { m.z = 2; 0 }; size(m); m", parse("{a:1 z:2}"));
	is!("m = {}; m[\"a\"] = 1; def count(m) { m.z = 2; 0 }; count(m); m.z", 2);
	is!("xs = [1]; def count(xs) { xs.add(2); 0 }; count(xs); xs", parse("[1 2]"));
}

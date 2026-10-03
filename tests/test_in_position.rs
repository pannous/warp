// D14 (user 2026-10-03): `x in list` gives the 1-based position of x (truthy when found), 0 when absent;
// round 3: "Never warn".
use warp::*;
use warp::diagnostic::{with_warning_mode, WarningMode};

#[test]
fn in_gives_the_one_based_position() {
	is!("3 in [1 2 3]", 3);
	is!("1 in [1 2 3]", 1);
	is!("5 in [1 2 3]", 0);
	is!("xs=[7 8 9]; 8 in xs", 2);
}

#[test]
fn a_found_first_element_is_truthy() {
	is!("if 1 in [1 2 3] {7} else {8}", 7);
	is!("if 5 in [1 2 3] {7} else {8}", 8);
	is!("xs=[4 5]; if 4 in xs {1} else {0}", 1);
}

#[test]
fn the_position_never_warns() {
	with_warning_mode(WarningMode::Error, || is!("3 in [1 2 3]", 3));
	with_warning_mode(WarningMode::Error, || is!("if 3 in [1 2 3] {7} else {8}", 7));
}

#[test]
fn maps_and_has_keep_their_truth_value() {
	is!("m={a:1 b:2}; \"b\" in m", 1);
	is!("xs=[7 8 9]; xs.has(9)", 1);
}

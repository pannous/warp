// D14 (user 2026-10-03) gave `x in list` the 1-based position; in-bool (user 2026-10-12) made it yes/no, the position
// comes from `xs.index_of(x)`. Round 3: "Never warn".
use crate::is;
use warp::errors::diagnostic::{with_warning_mode, WarningMode};

#[test]
fn in_gives_yes_or_no() {
	is!("3 in [1 2 3]", true);
	is!("1 in [1 2 3]", true);
	is!("5 in [1 2 3]", false);
	is!("xs=[7 8 9]; 8 in xs", true);
	is!("(2 in [5 2]) == yes", true);
	is!("xs=[7 8 9]; xs.index_of(8)", 2);
}

#[test]
fn a_found_first_element_is_truthy() {
	is!("if 1 in [1 2 3] {7} else {8}", 7);
	is!("if 5 in [1 2 3] {7} else {8}", 8);
	is!("xs=[4 5]; if 4 in xs {1} else {0}", 1);
}

#[test]
fn the_position_never_warns() {
	with_warning_mode(WarningMode::Error, || is!("3 in [1 2 3]", true));
	with_warning_mode(WarningMode::Error, || is!("if 3 in [1 2 3] {7} else {8}", 7));
}

#[test]
fn maps_and_has_keep_their_truth_value() {
	is!("m={a:1 b:2}; \"b\" in m", true);
	is!("xs=[7 8 9]; xs.has(9)", true);
}

// Card leading-gt: a comparison without its left side was dropped silently (`> 100` read as 100). In a match arm it
// compares the subject (C#'s relational pattern `> 100 =>`), anywhere else it is an error naming what is missing
use crate::is;
use crate::common::fails_with;

const SIZE: &str = "size(v) := match v {\n < 0 => \"negative\"\n >= 100 => \"big\"\n _ => \"small\" }\n";

#[test]
fn a_match_arm_compares_the_subject() {
	is!(&format!("{SIZE}size(-5)"), "negative");
	is!(&format!("{SIZE}size(150)"), "big");
	is!(&format!("{SIZE}size(100)"), "big");
	is!(&format!("{SIZE}size(50)"), "small");
}

#[test]
fn a_comparison_without_its_left_side_is_an_error() {
	fails_with("> 100", "compares nothing");
	fails_with("x = 3\n> 100", "compares nothing");
}

#[test]
fn the_operator_value_stays() {
	is!("xs = [3, 1, 2]; sorted(xs, >)#1", 3);
}

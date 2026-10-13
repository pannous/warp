// card match-negative-subject: `match -3 {…}` matches the number -3; it was parsed as the subtraction `match - 3`
use crate::is;

#[test]
fn a_negative_subject_is_matched() {
	is!("match -3 { -3 => \"minus three\"; _ => \"other\" }", "minus three");
	is!("switch -2 { -2 => 1; _ => 2 }", 1);
	is!("x := 4; match -x { -4 => 1; _ => 2 }", 1);
	is!("match #[1, 2] { 2 => \"two\"; _ => \"other\" }", "two");
}

#[test]
fn a_variable_named_match_still_subtracts() {
	is!("match := 5; match - 3", 2);
}

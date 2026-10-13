//! `x is [_, 2]` tests a pattern like a match arm does; `if xs is [a, b] then b` binds the pattern's names for the
//! branch (card is-pattern). A list of known values still compares
use crate::is;

#[test]
fn is_tests_a_pattern() {
	is!("xs = [1, 2]; xs is [_, 2]", true);
	is!("xs = [1, 3]; xs is [_, 2]", false);
	is!("xs = [1, 2, 3]; xs is [1, ...rest]", true);
	is!("xs = [1, 2, 3]; xs is [_, _]", false);
}

#[test]
fn if_is_binds_the_names() {
	is!("xs = [1, 2]; if xs is [a, b] then b else 0", 2);
	is!("xs = [1, 2, 3]; if xs is [a, b] then b else 0", 0);
	is!("xs = [5, 6, 7]; if xs is [first, ...rest] then first + #rest else 0", 7);
}

#[test]
fn known_values_still_compare() {
	is!("a = 1; b = 2; xs = [1, 2]; xs is [a, b]", true);
	is!("xs = [1, 2]; xs is [1, 2]", true);
}

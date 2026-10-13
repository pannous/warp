//! Ranges in a match arm (card match-ranges): `13 to 19` and `13...19` include their end, `13..20` excludes it (as
//! `count(1..3)` is 2), `65..` has no end; the arm compares the subject with the bounds, so floats and characters match
//! too. The arm used to compare the subject with the range's list and silently miss.
use crate::is;

#[test]
fn an_inclusive_range() {
	is!("match 19 { 13 to 19 => \"teen\"; _ => \"other\" }", "teen");
	is!("match 13 { 13...19 => \"teen\"; _ => \"other\" }", "teen");
	is!("match 20 { 13 to 19 => \"teen\"; _ => \"other\" }", "other");
}

#[test]
fn an_exclusive_range() {
	is!("match 15 { 13..20 => \"teen\"; _ => \"other\" }", "teen");
	is!("match 20 { 13..20 => \"teen\"; _ => \"other\" }", "other");
	is!("match 12 { 13..20 => \"teen\"; _ => \"other\" }", "other");
}

#[test]
fn a_range_without_end() {
	is!(
		"age(n) := match n { 0..13 => \"child\"; 13..20 => \"teen\"; 65.. => \"senior\"; _ => \"adult\" }; age(70)",
		"senior"
	);
	is!(
		"age(n) := match n { 0..13 => \"child\"; 13..20 => \"teen\"; 65.. => \"senior\"; _ => \"adult\" }; age(30)",
		"adult"
	);
}

#[test]
fn floats_and_ranges_as_alternatives() {
	is!("match 2.5 { 1 to 3 => \"low\"; _ => \"high\" }", "low");
	is!("match 7 { 1 to 3 | 6 to 9 => \"in\"; _ => \"out\" }", "in");
}

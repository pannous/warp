//! card rust-inclusive-range: Rust's `a..=b` is the inclusive range, as warp's `a…b` / `a to b` (decision
//! rust-forms-welcome), in a for header, a match arm and as a value
use crate::is;

#[test]
fn an_inclusive_range_in_a_for_header() {
	is!("let mut s = 0; for i in 1..=5 { s += i; } s", 15);
	is!("s = 0; for i in 1 ..= 3 { s += i }; s", 6);
}

#[test]
fn an_inclusive_range_in_a_match_arm() {
	is!("let n = 9; match n { 0..=9 => \"small\", _ => \"big\" }", "small");
	is!("let n = 50; match n { 0..=9 => \"small\", _ => \"big\" }", "big");
}

#[test]
fn an_inclusive_range_is_the_range_to_its_end() {
	is!("count(1..=4)", 4);
	is!("(1..=4) == (1…4)", true);
}

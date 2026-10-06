// #38 (user 2026-10-03, "Truthy"): a non-empty list is truthy, also one holding only ø.
use crate::is;

#[test]
fn a_list_of_nothing_is_truthy() {
	is!("not [ø]", false);
	is!("not ({[ø]})", false);
	is!("if [ø] {1} else {2}", 1);
	is!("x=[ø]; if x {1} else {2}", 1);
	is!("not []", true);
}

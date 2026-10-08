//! P204 (card item-unchecked): a loop item of a mixed list is of static type any, going into an annotated place it is
//! checked when it runs, as `n = xs#2` is
use crate::common::fails_with;
use crate::is;

#[test]
fn a_mixed_loop_item_into_an_annotated_place_is_checked_at_run_time() {
	fails_with("n: int = 0; for x in [1, \"a\"] { n = x }; n", "not an int");
	fails_with("t: text = \"\"; for x in [\"a\", 2] { t = x }; t", "not a text");
	is!("n: int = 0; for x in [1, 2] { n = x }; n", 2);
	is!("n: int = 0; for x in [1, \"a\"] { if x is int { n = x } }; n", 1);
}

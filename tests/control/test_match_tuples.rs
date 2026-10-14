//! Tuple arms `(1, t) =>` and `(a, b) =>` match by shape like list arms `[1, t]`: literals compare, `_` takes any item,
//! names bind (card match-tuples)
use crate::is;

#[test]
fn tuple_arm_binds_its_names() {
	is!("match (3, 4) { (a, b) => a + b }", 7);
	is!("match (1, 2) { (1, t) => t; _ => 0 }", 2);
}

#[test]
fn tuple_arms_pick_by_their_literals() {
	is!("p = (0, 5); match p { (0, y) => \"on y \" + y; (x, 0) => \"on x \" + x; _ => \"elsewhere\" }", "on y 5");
	is!("p = (4, 0); match p { (0, y) => \"on y \" + y; (x, 0) => \"on x \" + x; _ => \"elsewhere\" }", "on x 4");
	is!("match (2, 9) { (1, _) => \"one\"; (2, _) => \"two\"; _ => \"no\" }", "two");
}

#[test]
fn tuple_arm_checks_the_count() {
	is!("match (1, 2, 3) { (a, b) => \"two\"; (a, b, c) => \"three\" }", "three");
}

#[test]
fn literal_tuple_arm_still_compares() {
	is!("match (1, \"a\") { (1, \"a\") => \"yes\"; _ => \"no\" }", "yes");
}

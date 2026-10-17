//! Card match-static: a match on a subject the compiler knows is no instance (a bare variant, a number) leaves its
//! class arms dead; they are not typed with the subject's type (`s#r` of a symbol would be a character).
use crate::is;

const SHAPE: &str = "type Shape = Circle(r) | Rect(w, h) | Dot";

#[test]
fn a_bare_variant_skips_the_class_arms() {
	is!(&format!("{SHAPE}; s = Dot; match s {{ Circle(r) => 3 * r * r, Rect(w, h) => w * h, Dot => 0 }}"), 0);
	is!(&format!("{SHAPE}; s = Dot; match s {{ Circle(r) => r * r, _ => 7 }}"), 7);
}

#[test]
fn an_instance_still_takes_its_arm() {
	is!(&format!("{SHAPE}; s = Rect(2, 3); match s {{ Circle(r) => 3 * r * r, Rect(w, h) => w * h, Dot => 0 }}"), 6);
}

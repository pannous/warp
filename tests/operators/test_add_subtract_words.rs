//! card add-subtract: `add` and `subtract` are infix words only between operands, `2 add 3`; members `xs.add(…)`,
//! a program's own add, the phrase `add x to y` and `3 times {…}` keep their meaning
use crate::is;

#[test]
fn add_and_subtract_between_operands() {
	is!("2 add 3", 5);
	is!("5 subtract 2", 3);
	is!("x = 4; x add 1", 5);
	is!("2 times 3", 6);
}

#[test]
fn other_adds_keep_their_meaning() {
	is!("xs = [1]; xs.add(2); count(xs)", 2);
	is!("pixel=[1 2 3]; do add 4 to pixel; count(pixel)", 4);
	is!("add(a, b) := a * b; add(2, 3)", 6);
	is!("n = 0; 3 times { n = n + 1 }; n", 3);
}

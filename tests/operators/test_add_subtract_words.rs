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
	is!("n = 0; on subtract { n = 5 }; emit subtract; n", 5);
	is!("n = 0; when add { n = 5 }; emit add; n", 5);
	is!("n = 0; on plus { n = 5 }; emit plus; n", 5);
	is!("result = 2; result plus 1", 3);
}

/// One rule for every arithmetic word (reviewer): a program's own function of that name wins, a variable named like one
/// is an operand
#[test]
fn every_arithmetic_word_follows_one_rule() {
	for code in ["def add(a,b):=a*b; 2 add 3", "def minus(a,b):=a*b; 2 minus 3", "def plus(a,b):=a*b; 2 plus 3"] {
		assert!(warp::wasm_emitter::eval(code).serialize().contains("needs 2 arguments"), "{code}");
	}
	is!("add = 5; add subtract 1", 4);
	is!("5 subtract -2", 7);
	is!("2 add 3 times 4", 14);
	is!("7 div 2", 3);
}

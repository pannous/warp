// List slices in words (card word-slices, user 2026-10-09, notes/decisions.md): by position like xs#1 (first = 1),
// both ends inclusive; a position is marked, `#n` or an ordinal, a bare number is a loud error
use crate::is;
use warp::ints;

const XS: &str = "xs = [10, 20, 30, 40, 50]; ";

fn sliced(phrase: &str) -> String {
	format!("{XS}{phrase}")
}

#[test]
fn hash_positions_slice() {
	is!(&sliced("xs from #2 to #4"), ints(vec![20, 30, 40]));
	is!(&sliced("xs starting from #2"), ints(vec![20, 30, 40, 50]));
	is!(&sliced("xs up to #2"), ints(vec![10, 20]));
	is!(&sliced("n = 3; xs from #n to #5"), ints(vec![30, 40, 50]));
	is!(&sliced("ys = xs up to #2; count ys"), 2);
	is!(&sliced("count(xs up to #3)"), 3);
	is!(&sliced("f(x) := count x; f(xs from #2 to #3)"), 2);
}

#[test]
fn ordinal_positions_slice() {
	is!(&sliced("xs from second to fourth"), ints(vec![20, 30, 40]));
	is!(&sliced("xs up to second"), ints(vec![10, 20]));
	is!(&sliced("xs from 2nd to 4th"), ints(vec![20, 30, 40]));
	is!(&sliced("xs starting from 3rd"), ints(vec![30, 40, 50]));
	is!(&sliced("n = 4; xs from (n)th to last"), ints(vec![40, 50]));
	is!(&sliced("n = 2; xs up to nth"), ints(vec![10, 20]));
	is!("s = \"abcdef\"; s from #2 to #4", "bcd");
}

/// `xs up to 2` may mean the item 2: the error names both position forms
#[test]
fn a_bare_number_is_no_position() {
	crate::common::fails_with(&sliced("xs up to 2"), "xs up to #2 or xs up to second");
	crate::common::fails_with(&sliced("xs from 2 to 4"), "xs from #2 to #4 or xs from second to fourth");
}

/// other uses of from and to keep their meaning
#[test]
fn other_from_and_to_stay() {
	is!("count(1 to 3)", 3);
	is!("to move x from a to b { print \"\\(x) \\(a)→\\(b)\" }; move 1 from 2 to 3; 7", 7);
}

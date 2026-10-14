//! Operators, indexing, counting and iteration of a foreign value forward to its runtime's operator module
//! (`a * 2` → operator.mul(a, 2), `a#2` → operator.getitem(a, 1), `count a`, `for x in a`; lowering/foreign_modules.rs)
use crate::is;

#[cfg(feature = "native")] // python3
#[test]
fn test_python_operators() {
	is!("use python \"datetime\"; d = datetime.date(2020, 1, 2); e = datetime.date(2020, 1, 5); (e - d).days", 3);
	is!("use python \"fractions\"; f = fractions.Fraction(1, 2); (f * 2).numerator", 1);
	is!("use python \"fractions\"; f = fractions.Fraction(1, 2); f < 1", 1);
}

#[cfg(feature = "native")]
#[test]
fn test_python_indexing_counting_iteration() {
	is!("use python \"collections\"; d = collections.deque([1, 2, 3]); d#2", 2);
	is!("use python \"collections\"; d = collections.deque([1, 2, 3]); count d", 3);
	is!("use python \"collections\"; d = collections.deque([1, 2, 3]); s = 0; for x in d { s += x }; s", 6);
}

#[cfg(feature = "native")] // node's Buffer
#[test]
fn test_js_indexing_and_counting() {
	is!("use js Buffer; b = Buffer.from(\"abc\"); count b", 3);
	is!("use js Buffer; b = Buffer.from(\"abc\"); b#1", 97);
}

#[test]
fn test_plain_values_keep_their_operators() {
	is!("x = 3; -x * 2", -6);
}

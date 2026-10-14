// Card natural-phrases (samples/natural.warp), the phrases of one obvious meaning: operator words as aliases
// (wiki/operator.md lists `plus` and `equals`), `is in` for `in`, `through` for `to`, `for each x in xs`
use crate::is;

#[test]
fn operator_words() {
	is!("1 plus 2", 3);
	is!("5 minus 2", 3);
	is!("6 divided by 2", 3);
	is!("x = 1; y = 1; x equals y", 1);
	is!("a = 3; b = 2; a is greater than b", 1);
	is!("a = 3; b = 2; a less than b", 0);
	is!("age = 20; age is at least 18", 1);
	is!("age = 20; age is at most 18", 0);
}

#[test]
fn a_function_of_an_operator_word_stays_callable() {
	is!("plus(a, b) := a + b; plus(1, 2)", 3);
}

#[test]
fn membership_ranges_and_loops() {
	is!("item = 2; collection = [1, 2]; if item is in collection { 1 } else { 0 }", 1);
	is!("s = \"\"; for c in 'a' through 'c' { s = s + c }; s", "abc");
	is!("total = 0; basket = [{price: 2}, {price: 3}]; for each item in basket { total = total plus item.price }; total", 5);
	is!("s = 0; for each in [1, 2] { s = s + each }; s", 3);
}

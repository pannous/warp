// card match-method: `match [1, 2, 3].reverse() {…}` matches the method call's value like `match xs.reverse() {…}`;
// a receiver that is a bracketed group, a call or a text left the block the body of a definition of reverse
use crate::is;

#[test]
fn a_method_of_a_literal_is_the_subject() {
	is!("match [1, 2, 3].reverse() { [3, 2, 1]: \"reversed\"; _: \"other\" }", "reversed");
	is!("twice(x) := x * 2\nmatch [1, 2, 3].map(twice) { [2, 4, 6]: \"doubled\"; _: \"other\" }", "doubled");
	is!("match \"abc\".upper() { \"ABC\": 1; _: 2 }", 1);
	is!("f(x) := [x, x]\nmatch f(2).reverse() { [2, 2]: \"pair\"; _: \"other\" }", "pair");
}

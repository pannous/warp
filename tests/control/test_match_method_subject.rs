// card match-xs: `match xs.map(f) {…}` matches the method call's value; the block was read as the body of a definition
// of map, so `match` was left an undefined variable
use crate::is;

#[test]
fn a_method_call_is_the_subject() {
	is!("xs = [1, 2, 3]\ntwice(x) := x * 2\nmatch xs.map(twice) {\n\t[2, 4, 6]: \"doubled\"\n\t_: \"other\"\n}", "doubled");
	is!("t = \"abc\"\nswitch t.upper() { \"ABC\": 1; _: 2 }", 1);
}

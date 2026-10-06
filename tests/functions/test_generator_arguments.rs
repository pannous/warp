// Python's generator expression as the one argument of a call: `sum(x * x for x in xs)` is `sum([x * x for x in xs])`
use crate::is;

#[test]
fn a_generator_argument() {
	is!("sum(x * x for x in [1, 2, 3])", 14);
	is!("max(x % 5 for x in [3, 9, 12])", 4);
	is!("any(x > 2 for x in [1, 3])", 1);
	is!("def f(xs){ count(xs) }; f(x for x in [1, 2, 3] if x > 1)", 2);
}

#[test]
fn any_and_all_of_a_list() {
	is!("any([0, 3])", 1);
	is!("all([1, 0])", 0);
}

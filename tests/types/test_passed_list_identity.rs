//! A list passed through a function is the same list (P208), also one the emitter would keep as a typed array: a
//! variable compared by identity stays a Node list, so both sides are the one object (card xs-xs)
use crate::is;

#[test]
fn test_list_through_a_function_is_the_same() {
	is!("f(a) := a; xs = [1, 2]; f(xs) === xs", true);
	is!("f(a) := a; xs: ints = [1, 2]; f(xs) === xs", true);
	is!("f(a) := a; xs = [1, 2]; xs#1 = 5; f(xs) === xs", true);
	is!("f(a) := a; xs = [1 2]; xs.add(3); xs same f(xs)", true);
	is!("f(a) := a; xs = [1, 2]; f(xs) !== xs", false);
	is!("f(a) := [1, 2]; xs = [1, 2]; f(xs) === xs", false);
}

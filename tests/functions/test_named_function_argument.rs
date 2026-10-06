// Python's `sorted(xs, key=x => -x)`: a function passed as `name = …` is a named argument like `key: x => -x` (P37);
// it was lowered to the definition (key x) := -x and sort said it needs a function
use crate::is;

#[test]
fn a_function_assigned_in_a_call_is_a_named_argument() {
	is!("xs = [3, 1, 2]; sorted(xs, key=x=>-x)#1", 3);
	is!("sorted([3, 1, 2], key = x => -x)#3", 1);
	is!("sorted([3, 1, 2], key: x => -x)#1", 3);
	is!("f = x => x * 2; f(4)", 8);
}

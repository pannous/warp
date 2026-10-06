// Ruby calls without parentheses: the arguments after a comma belong to a call that is short of them,
// `add 1, 2` is `add(1, 2)` (it read as the list `add(1), 2`: 'add needs a value for parameter b')
use crate::is;

#[test]
fn comma_arguments_of_a_short_call() {
	is!("def add(a, b){ a+b }; add 1, 2", 3);
	is!("def add(a, b){ a+b }; x = add 1, 2; x", 3);
	is!("def add3(a, b, c){ a+b+c }; add3 1, 2, 3", 6);
	is!("def add(a, b){ a+b }; add 1 2", 3);
}

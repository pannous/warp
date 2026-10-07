// card count-shadowed: a variable named like a counting word (count, size, length) and applied to a value is a loud
// compile error naming the variable, on every path; before, `"users: " + count(users)` built the list (0 users) and
// trapped in the join, while `n = count(users)` ignored the variable and counted
use crate::common::fails_with;
use crate::is;

const SHADOWED: &str = "`count` is a variable here, so count(users) cannot count: rename the variable";

#[test]
fn a_variable_named_count_applied_to_a_value_is_an_error() {
	fails_with("count = 0; users = [\"a\"]; \"users: \" + count(users)", SHADOWED);
	fails_with("count = 0; users = [\"a\"]; n = count(users); n", SHADOWED);
	fails_with("count = 0; users = [\"a\"]; count(users)", SHADOWED);
}

#[test]
fn count_stays_a_word_and_a_variable_name() {
	is!("users = [\"a\", \"b\"]; count(users)", 2);
	is!("count = 2; count + 1", 3);
	is!("counter = 0; users = [\"a\"]; \"users: \" + count(users)", "users: 1");
}

// a global `count` (a function reads it) leaves the word to the functions: lib/markup.wasp's count(items) under a
// program's `count = 0`
#[test]
fn a_global_count_does_not_hide_the_word_in_functions() {
	is!("count = 5\ntwice() := count * 2\nsized(xs) := count(xs)\nsized([1, 2, 3]) + twice()", 13);
	is!("count = 5\ntwice() := count * 2\nlabel(xs) := \"n \" + count(xs)\nlabel([1, 2, 3])", "n 3");
}

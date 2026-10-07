// card contains-infix: `xs contains x` (and `has`, `includes`) is `xs.contains(x)` written between its values, binding
// like `x in xs`, so it works in conditions; a program's own word of that name keeps its meaning
use crate::is;

#[test]
fn contains_between_its_values() {
	is!("e = \"abcd\"; e contains \"bc\"", true);
	is!("xs = [1, 2]; xs has 3", false);
	is!("m = {a: 1}; m includes \"a\"", true);
}

#[test]
fn contains_in_a_condition() {
	is!("e = \"abcd\"; if e contains \"bc\" then 1 else 2", 1);
	is!("xs = [1, 2]; xs contains 2 and not (xs includes 5)", true);
}

#[test]
fn a_variable_named_like_the_word_stays_a_variable() {
	is!("has = 3; has + 1", 4);
	is!("p = {has: 2}; p.has", 2);
}

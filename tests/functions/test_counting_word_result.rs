// `count ys` without parentheses gives a number, also as a function's result: g's result was taken for a list
use crate::is;

#[test]
fn a_counting_word_gives_a_number() {
	is!("g(ys) := count ys; f(xs) := g(xs) + 1; f([1,2,3,4])", 5);
	is!("g(ys) := count ys; g([1,2,3]) + 1", 4);
	is!("g(ys) := length ys; g([1,2]) * 2", 4);
	is!("g(ys) := size ys; g(\"abc\") + 1", 4);
	is!("xs = [1,2]; y = count xs; y + 1", 3);
}

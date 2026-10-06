// Recursion over a list as Python, Haskell, Ruby and JS write it: `xs[0] + f(xs[1:])`, the element's kind known only at
// run time; was a trap (the sum was taken for text: the branches 0 and element+recursion judged a Text)
use crate::is;

#[test]
fn a_recursive_sum_over_slices() {
	is!("def s(xs){ #xs == 0 ? 0 : xs#1 + s(xs[1:]) }; s([1,2,3])", 6);
	is!("def s(xs){ if #xs == 0 {0} else { xs#1 + s(xs[1:]) } }; s([1,2,3])", 6);
	is!("def p(xs){ #xs == 0 ? 1 : xs#1 * p(xs[1:]) }; p([2,3,4])", 24);
	is!("def s(xs, i){ i > #xs ? 0 : xs#i + s(xs, i+1) }; s([1,2,3], 1)", 6);
}

#[test]
fn a_recursive_maximum() {
	is!("def m(xs){ #xs == 1 ? xs#1 : max(xs#1, m(xs[1:])) }; m([3,9,2])", 9);
}

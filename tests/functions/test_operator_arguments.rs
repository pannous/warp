// An operator alone as an argument is the operator as a value, like `by: >`: `sorted(xs, >)` dropped the `>`
use crate::is;

#[test]
fn an_operator_argument_is_the_operator() {
	is!("xs = [3, 1, 2]; sorted(xs, >)#1", 3);
	is!("xs = [3, 1, 2]; sorted(xs, <)#1", 1);
	is!("xs = [3, 1, 2]; xs.sort(>)#1", 3);
	is!("reduce([1, 2, 3], +)", 6);
}

#[test]
fn a_word_operator_alone_stays_a_name() {
	is!("hanoi(n, from, to, via) := n + to; hanoi(1, 2, 3, 4)", 4);
	is!("f(a, to) := to; f(1, 5)", 5);
}

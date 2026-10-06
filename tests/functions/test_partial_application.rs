// `add(1, _)` is the lambda of the left-out arguments (samples/functions.wasp), and a zero-argument closure is called
use warp::ints;
use crate::is;

#[test]
fn a_placeholder_makes_a_partial_application() {
	is!("def add(a, b) := a + b; increment = add(1, _); increment(4)", 5);
	is!("f(a,b,c):=a*100+b*10+c; g = f(_, 2, _); g(1, 3)", 123);
	is!("xs=[1 2 3]; mul(a,b):=a*b; map(xs, mul(2, _))", ints(vec![2, 4, 6]));
}

#[test]
fn a_returned_zero_argument_closure_is_called() {
	is!("def counter(start) { count = start; return () => { count = count + 1; return count } }; c = counter(5); c()", 6);
	is!("mk(n) := { return x => x + n }; f = mk(2); f(5)", 7);
}

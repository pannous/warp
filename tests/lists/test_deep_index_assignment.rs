// `xs#i = v` deep in a list of Nodes copies the cells before i without recursion (card list-recursion): 90000 cells
// deep exhausted the call stack
use crate::is;

#[test]
fn a_deep_index_assignment_keeps_the_stack() {
	// a global list stays a list of Nodes (no typed array)
	is!("f(n) := { r = []; for i in 1 to n { r.add(i) }; r }; global xs = f(100000); xs#90000 = 5; [xs#90000, xs#89999, xs#1]", warp::ints(vec![5, 89999, 1]));
	is!("xs = [1, 2, 3]; ys = xs; ys#3 = 9; [xs#3, ys#3, ys#1]", warp::ints(vec![3, 9, 1]));
}

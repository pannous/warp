// A list mixing floats with exact numbers is a `list of number`: its elements were read back as ints ("not an int" for
// `xs = [sqrt(2.0), 0.2]; xs[0] + 1`); each is held as its Node now, of its own kind (found by card winamp-like)
use crate::is;

#[test]
fn a_mixed_number_list_keeps_each_elements_kind() {
	is!("xs = [sqrt(2.0), 0.25]\nxs[1] + 1", 1.25);
	is!("xs = [sqrt(4.0), 0.25]\nmax(xs[0], 0.1)", 2.0);
	is!("xs = [1.5, 2.5f]\nxs[1] + 1", 3.5);
	is!("x:numbers=[1 2.5f]\nx[1] + 1", 3.5);
	is!("x:numbers=[1 2 3]\nx[1] + 1", 3);
}

// The float elements of a comprehension or of a function's list, read in a loop body or passed to a function that also
// gets ints, were unboxed as ints ("not an int", samples/visualizer.warp); a loop keeps such an element as its Node, and a
// parameter passed Nodes and ints takes the Node; == compares a Node by value
#[test]
fn float_elements_stay_floats_in_loops_and_calls() {
	is!("xs = [i / 2.0 for i in 0..3]\nfor k in 0..2 { xs[k] }", 0.5);
	is!("xs = [sqrt(i + 0.5) for i in 0..4]\nfor k in 0..2 { xs[1] }\n3", 3);
	is!("def halves(n) { [i / 2.0 for i in 0..n] }\nxs = halves(4)\nfor k in 0..4 {\n\txs[k]\n\t3\n}", 3);
	is!("def halves(n) { [i / 2.0 for i in 0..n] }\ndef plus(a, b) { a + b }\nxs = halves(4)\nplus(1, 2)\nplus(1, xs[3])", 2.5);
	// a function summing a list parameter's elements with floats gives a Node: == compares its value
	is!("def wave(w) {\n\tre = 0.0\n\tfor i in 0..count(w) { re = re + (w[i] - 5) * cos(1.0) }\n\tre\n}\nwave([5, 5, 5]) == 0", true);
}

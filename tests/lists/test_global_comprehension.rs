// card global-comprehension: a global list built by a comprehension has the comprehension's element type, so
// writing and reading it works as for a literal list (it was a cast failure)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_global_comprehension_list_takes_writes() {
	is!("global hb = [random() * 0.0 + 0.5 for j in 0..2]; hb[1] = 0.25; hb[1] + 1", 1.25);
	is!("global hb = [random() * 0.0 + 0.5 for j in 0..2]; hb[1] = hb[1] + 0.5; hb[1]", 1.0);
	is!("hb = [random() * 0.0 + 0.5 for j in 0..2]; def g() { global hb; hb[1] = 0.25 }; g(); hb", parse("[0.5 0.25]"));
}

// card float-into-nodes: a function writing a machine float into a global list held as Nodes (node_with_at takes an
// i64 value: 'not an int'); node_with_node_at stores the Node
#[test]
fn a_function_writes_a_float_into_a_global_list() {
	is!("hb = [random() * 0.0 + 0.5 for j in 0..2]; def f() { t = hb[1]; t + 1 }; def g() { global hb; hb[1] = hb[1] + 0.5 }; g(); f()", 2.0);
}

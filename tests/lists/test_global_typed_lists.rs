// A list main builds and functions read is a global; a global list of numbers is a typed array like a local one
// (card compiler-picks): indexing it in a function is O(1), building it by appends is amortised O(1)
use crate::is;
use warp::{int, ints, list};

#[test]
fn a_global_number_list_read_by_a_function_is_an_array() {
	// as cons cells: 10^5 appends and 10^5 indexed reads ran out of fuel
	is!("n = 100000; xs = []; for i in 1..n { xs.add(i) }; at(i) := xs#i; s = 0; for i in 1..n { s += at(i) }; s", 4999950000i64);
	is!("xs = [1, 2, 3]; size() := count xs; total() := sum xs; [size(), total()]", ints(vec![3, 6]));
	is!("xs = int[5]; for i in 1 to 5 { xs#i = i * i }; f(i) := xs#i; f(4)", 16);
	is!("xs = []; for i in 1 to 4 { xs.add(i) }; doubled() := { s = 0; for x in xs { s += 2 * x }; s }; doubled()", 20);
}

#[test]
fn a_global_typed_list_reads_as_its_list() {
	is!("xs = [1, 2, 3]; f() := xs; f()", ints(vec![1, 2, 3]));
	is!("xs = [1, 2]; xs.add(3); g() := count xs; [g(), xs]", list(vec![int(3), ints(vec![1, 2, 3])]));
	is!("xs = [1, 2, 3]; ys = xs; ys#1 = 9; f() := xs#1 + ys#1; f()", 10);
}

/// a function that changes the global list keeps it a Node list, as before
#[test]
fn a_global_list_a_function_changes_stays_as_before() {
	is!("global xs = [1, 2, 3]; bump() := { xs#1 = 7 }; bump(); xs#1", 7);
	is!("global xs = []; push(v) := { xs.add(v) }; push(4); push(5); xs", ints(vec![4, 5]));
	is!("global xs = [1, 2, 3]; f(i) := xs#i; f(2)", 2);
}

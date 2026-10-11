use crate::is;

const DELETE_FROM_LIST: &str = include_str!("../../probes/delete_list.warp");

#[test]
fn delete_removes_the_first_matching_list_item() {
	is!(DELETE_FROM_LIST, warp::ints(vec![1, 3, 2]));
}

// `delete xs[i]`, `delete xs#i`, `delete m["k"]`, `delete m.k` remove the entry, as Python's `del` (lowered to pop)
#[test]
fn delete_statement_removes_an_entry() {
	is!("xs = [1 2 3]; delete xs[1]; xs", warp::ints(vec![1, 3]));
	is!("xs = [1 2 3]; delete xs#1; xs", warp::ints(vec![2, 3]));
	is!("xs = [1 2 3]; ys = xs; delete xs[0]; ys", warp::ints(vec![2, 3]));
	is!("m = {a:1 b:2}; delete m[\"a\"]; m.b * 10 + count(m)", 21);
	is!("m = {a:1 b:2}; delete m.b; m.a * 10 + count(m)", 11);
}

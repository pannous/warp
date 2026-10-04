// Small helpers are inlined (inlining.rs), so a list they update by index stays the caller's array; values stay values
use warp::{ints, is};

const SWAP: &str = "def swap(arr, i, j) { temp = arr[i]; arr[i] = arr[j]; arr[j] = temp; return arr }; ";

#[test]
fn an_inlined_helper_updates_the_list_handed_back() {
	is!(&format!("{SWAP}x=[1 2 3]; x = swap(x, 0, 2); x"), ints(vec![3, 2, 1]));
}

#[test]
fn an_inlined_helper_leaves_its_argument_unchanged() {
	is!(&format!("{SWAP}x=[1 2 3]; y = swap(x, 0, 2); x#1 * 10 + y#1"), 13);
}

#[test]
fn a_helper_with_free_variables_or_recursion_is_called() {
	is!("k=5; add(x) := { y = x + k; return y }; add(1)", 6);
	is!("f(n) := { if n < 1 { return 0 }; return n + f(n-1) }; f(4)", 10);
}

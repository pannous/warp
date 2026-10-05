// A group that runs statements, `(y=1; y)`, or a print inside a block runs; the block's value is its last item,
// also when that item is a list (was the data block `{1 [4 5]}`, or ø after a print)
use warp::{ints, is};

#[test]
fn a_statement_group_in_a_function_body_runs() {
	is!("def f(xs){ (y=1; y); xs }; f([4 5])", ints(vec![4, 5]));
	is!("def f(xs){ (y=1); xs }; f([4 5])", ints(vec![4, 5]));
}

#[test]
fn a_print_in_a_block_runs_before_its_list_value() {
	is!("x=[1 2]; if 1 { print(1); x }", ints(vec![1, 2]));
	is!("def f(xs){ print(1)\nxs }; f([4 5])", ints(vec![4, 5]));
}

#[test]
fn a_list_result_after_a_print_is_read() {
	is!("print(1); [2 3]", ints(vec![2, 3]));
}

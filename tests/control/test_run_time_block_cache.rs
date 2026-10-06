//! Card bang-cache: a block run at run time is compiled as a function of the numbers it reads, so the same block with
//! other values is the same module: a loop that runs `xs#1!` with a changing variable compiles the block once
//! (notes/runtime_eval.md "Measured"). Natively; the browser compiles every block again.
use crate::is;

#[test]
#[cfg(feature = "native")]
fn a_block_run_in_a_loop_with_changing_numbers_compiles_once() {
	let before = warp::pipeline::compiled_blocks();
	is!("xs = [data a*2 + f, data 0]; s = 0; a = 0; f = 0.5; for i in 1..20 { a = i; f = f + 1; s += xs#1! }; s", 579.5);
	assert_eq!(warp::pipeline::compiled_blocks() - before, 1, "the block compiled once for 19 runs");
}

#[test]
fn a_cached_block_sees_the_values_of_each_run() {
	is!("xs = [data a*10 + b, data 0]; a = 0; b = 0; r = []; for i in 1..3 { a = i; b = i + 1; r = r + [xs#1!] }; r", warp::ints(vec![12, 23]));
	is!("xs = [data t + \"!\", data 0]; t = \"a\"; u = xs#1!; t = \"b\"; v = xs#1!; u + v", "a!b!");
}

/// Card run-time-blocks: a block run in a loop body sees the variables assigned before it in that body, and the loop's
/// own variable
#[test]
fn a_block_in_a_loop_body_sees_the_variables_assigned_there() {
	is!("xs = [data a*2, data 0]; s = 0; for i in 1..4 { a = i; s += xs#1! }; s", 12);
	is!("xs = [data i*10, data 0]; s = 0; for i in 1..4 { s += xs#1! }; s", 60);
	is!("xs = [data b+1, data 0]; r = 0; if 1 { b = 41; r = xs#1! }; r", 42);
}

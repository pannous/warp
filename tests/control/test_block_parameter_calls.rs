//! Card repeats-loop: `def repeats(n: int, block) := { for i in 1...n { block() } }; repeats 4 { … }` ran the block once,
//! at the call: a block parameter takes its argument unevaluated in every call form, and `block()` runs it like `block!`
use crate::is;

#[test]
fn a_block_parameter_takes_a_trailing_block_without_parentheses() {
	is!("def repeats(n: int, body: block) := { for i in 1...n { body! } }; k = 0; repeats 4 { k += 1 }; k", 4);
}

#[test]
fn a_parameter_named_block_is_a_block_parameter_and_runs_with_parentheses() {
	is!("def repeats(n: int, block) := { for i in 1...n { block() } }; k = 0; repeats 3 { k += 1 }; k", 3);
}

#[test]
fn a_trailing_block_parameter_takes_the_rest_of_the_call() {
	is!("twice(body:block) := { body!; body! }; n = 0; twice(n += 1); n", 2);
	is!("twice(body:block) := { body!; body! }; n = 0; twice(n += 2 * 3); n", 12);
}

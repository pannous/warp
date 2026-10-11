//! Card repeats-loop: `def repeats(n: int, block) := { for i in 1...n { block() } }; repeats 4 { … }` ran the block once,
//! at the call: a block parameter takes its argument unevaluated in every call form, and `block()` runs it like `block!`
use crate::common::printed;
use crate::is;

const REPEATS: &str = "def repeats(n: int, body: block) := { for i in 1...n { body! } }\n";

#[test]
fn a_block_parameter_takes_a_trailing_block_without_parentheses() {
	assert_eq!(printed(&format!("{REPEATS}repeats 4 {{ print \"hi\" }}")), "hi\nhi\nhi\nhi\n");
}

#[test]
fn a_parameter_named_block_is_a_block_parameter_and_runs_with_parentheses() {
	let program = "def repeats(n: int, block) := { for i in 1...n { block() } }\nrepeats 3 { print \"hi\" }";
	assert_eq!(printed(program), "hi\nhi\nhi\n");
}

#[test]
fn a_trailing_block_parameter_takes_the_rest_of_the_call() {
	is!("twice(body:block) := { body!; body! }; n = 0; twice(n += 1); n", 2);
	assert_eq!(printed("twice(body:block) := { body!; body! }; twice(print \"hi\")"), "hi\nhi\n");
}

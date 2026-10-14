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

/// card parameter-named: a parameter named code or block that the body never runs is a value (self-hosting's lexers
/// read a `code` text), `second(s.code)` had given "code"+1
#[test]
fn a_parameter_named_code_the_body_never_runs_is_a_value() {
	is!("def second(code) := code[1]; second(\"abc\")", 'b');
	is!("class Source { code: text }; s = Source{code: \"xyz\"}; def second(code) := code[1]; second(s.code)", 'y');
	is!("def size(block) := count(block); size([1, 2, 3])", 3);
}

// card phrase-colon: the body after `to name params:` is the rest of the statement, a call without parentheses
// included (`print address`), and a block whose one statement is a call (`{ print b }`) returns that call's value
use crate::is;

#[test]
fn a_colon_body_is_the_whole_statement() {
	is!("to tell x to ys: count ys; tell 1 to [4, 5, 6]", 3);
	is!("to move x from a to b: x+a*b; move 1 from 2 to 3", 7);
}

#[test]
fn a_block_of_one_call_returns_the_calls_value() {
	is!("to move x from a to b { print b }; move 1 from 2 to 3; 5", 5);
	is!("f(b) := { print b }; f 3; 4", 4);
}

#[cfg(feature = "native")]
#[test]
fn a_colon_body_prints_its_argument() {
	assert_eq!(crate::common::printed(r#"to mail x to address: print address; mail "hi" to "a@b""#).trim(), "a@b");
	assert_eq!(crate::common::printed("to move x from a to b { print b }; move 1 from 2 to 3").trim(), "3");
}

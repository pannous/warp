//! An Error value used as a number fails with its own message, not "not a number": `x:int?=ø; x!+1` is "unwrapped ø"
use crate::common::fails_with;
use crate::is;

#[test]
fn test_unwrapping_an_empty_optional_in_arithmetic_names_it() {
	fails_with("x:int?=ø; x!+1", "unwrapped ø");
	is!("x:int?=5; x!+1", 6);
}

#[test]
fn test_an_error_value_in_node_arithmetic_keeps_its_message() {
	fails_with("f(n) := if n < 0 then error(\"negative\") else \"ok\"; x = f(-1); y = x + 1; y", "negative");
}

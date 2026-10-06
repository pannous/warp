use crate::is;
use crate::common::fails_with;

#[test]
fn test_object_variable_is_looked_up_by_key() {
	is!("o={a:1 b:2};o[b]", 2);
}

#[test]
fn test_key_is_evaluated() {
	is!("o={a:1 b:2};k=b;o[k]", 2);
}

#[test]
fn test_looked_up_value_takes_part_in_arithmetic() {
	is!("{a:1 b:2}[b]+1", 3);
}

#[test]
fn test_text_value_is_looked_up() {
	is!("{a:'x' b:'y'}[b]", "y");
}

#[test]
fn test_missing_key_is_an_error() {
	fails_with("{a:1 b:2}[c]", "key not found");
}

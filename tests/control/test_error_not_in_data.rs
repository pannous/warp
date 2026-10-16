//! Data stays clean (decision error-optional): an Error written into a list item raises, with its own message; results,
//! locals and arguments carry it
use crate::common::fails_with;
use crate::is;

const E: &str = "e = try 10/0; ";

#[test]
fn test_an_error_in_a_list_literal_raises() {
	fails_with(&format!("{E}xs = [e 2]; xs"), "divide by zero: a list item holds no error");
	fails_with(&format!("{E}f(x) := x; xs = [1 f(e)]; xs"), "a list item holds no error");
}

#[test]
fn test_a_list_of_values_and_a_handled_error_stay() {
	is!("f(x) := x; xs = [f(1) 2]; xs#1", 1);
	is!(&format!("{E}xs = [(try e else 0) 2]; xs#1"), 0);
}

#[test]
fn test_try_catches_an_error_in_data() {
	is!(&format!("{E}r = try [e 2]; if r failed then 1 else 2"), 1);
}

#[test]
fn test_an_error_still_travels_as_an_argument_and_a_local() {
	is!(&format!("{E}f(x) := if x failed then 1 else 2; f(e)"), 1);
	is!(&format!("{E}g = e; if g failed then 1 else 2"), 1);
}

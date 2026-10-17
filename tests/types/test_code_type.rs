//! Card body-should (decision body-code): a quoted program part, a function's body or `parse("…")`, has the type
//! code and prints its source; a braced body `{x; x+1}` is code, never a map
use crate::is;

#[test]
fn a_function_body_is_code() {
	is!("def f(x):=x+1; type f.body", "code");
	is!("def f(x):=x; type f.body", "code");
	is!("f(x):={x; x+1}; type f.body", "code");
	is!("def f(x):=x+1; type of ((function f).body)", "code");
	is!("def f(x):=x+1; b = f.body; type b", "code");
}

#[test]
fn a_function_body_tests_as_code() {
	is!("def f(x):=x+1; f.body is code", true);
	is!("def f(x):=x; f.body is code", true);
	is!("f(x):={x; x+1}; f.body is code", true);
	is!("f(x):={x; x+1}; f.body is map", false);
	is!("def f(x):=x+1; f.body is operation", true);
}

#[test]
fn a_function_body_prints_its_source() {
	is!("def f(x):=x+1; string(f.body)", "x+1");
	is!("f(x):={x; x+1}; string(f.body)", "{x; x+1}");
}

#[test]
fn parsed_code_is_code() {
	is!("type parse(\"1+1\")", "code");
	is!("parse(\"1+1\").type", "code");
	is!("parse(\"1+1\") is code", true);
	is!("parse(\"1+1\").op", "+");
	is!("1+1 is code", false);
	is!("3 is code", false);
}

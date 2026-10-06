//! A parameter shaped as an object (wiki/argument.md, card wiki-argument): `to call person{name?, phone number, email?,
//! mutable status} do …` takes an object with those fields; `?` marks an optional one, a missing required field of an
//! object written at the call is a compile error
use crate::common::fails_with;
use crate::is;

#[test]
fn test_shaped_parameter_reads_its_fields() {
	is!("f(p{name, title?}) := \"Hi \" + p.name; f({name:\"Ann\"})", "Hi Ann");
	is!("def f(p:{name}) { p.name }; f({name:\"Ann\"})", "Ann");
}

#[test]
fn test_to_definition_with_a_shaped_parameter() {
	is!("to greet person{name, title?} do \"Hi \" + person.name\ngreet {name:\"Ann\"}", "Hi Ann");
	is!("to greet p do \"Hi \" + p.name\ngreet {name:\"Ann\"}", "Hi Ann");
}

#[test]
fn test_wiki_argument_example() {
	let code = "to ring person{name?, phone number, email?, mutable status} do\n  person.status = \"called \" + person.phone\n  person.status\nring {name:\"Jonathan\" phone:\"899-573-5842\" status:\"new\"}";
	is!(code, "called 899-573-5842");
}

#[test]
fn test_missing_required_field_is_a_compile_error() {
	fails_with("f(p{name, phone}) := p.name; f({name:\"Dick\"})", "missing argument field phone");
	is!("f(p{name?, phone}) := p.phone; f({phone:\"8\"})", "8");
}

#[test]
fn test_constructed_argument_is_checked() {
	let call = "class person{name, phone?}\nto call p{name?, phone} do p.phone\n";
	fails_with(&format!("{call}call person{{name:\"Dick\"}}"), "missing argument field phone");
	fails_with(&format!("{call}call person {{name:\"Dick\"}}"), "missing argument field phone");
	is!(&format!("{call}call person{{name:\"J\" phone:\"8\"}}"), "8");
}

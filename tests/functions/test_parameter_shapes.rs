//! A parameter shaped as an object (wiki/argument.md, card wiki-argument): `to call person{name?, phone text, email?,
//! mutable status} do …` takes an object with those fields; `?` marks an optional one, a missing required field of an
//! object written at the call is a compile error, as is a literal of another type than its field's (P164)
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
	let code = "to ring person{name?, phone text, email?, mutable status} do\n  person.status = \"called \" + person.phone\n  person.status\nring {name:\"Jonathan\" phone:\"899-573-5842\" status:\"new\"}";
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

#[test]
fn test_field_type_is_checked() {
	fails_with("to call person{name?, phone number} do person.phone\ncall {phone:\"899-573-5842\"}", "type error: field phone is number");
	is!("to call person{name?, phone number} do person.phone + 1\ncall {phone:41}", 42);
	is!("f(p{name: text}) := p.name; f({name:\"Ann\"})", "Ann");
}

#[test]
fn test_hyphenated_field_with_a_type() {
	// P164 addendum (user): `phone-number:text` is one field name with its type
	is!("f(p{phone-number:text}) := p.phone-number; f({phone-number:\"899-573-5842\"})", "899-573-5842");
	is!("f(p{phone-number:text}) := p.phone-number; f({phone-number:\"8\"})", "8");
	fails_with("f(p{phone-number:text}) := p.phone-number; f({phone-number:8})", "type error: field phone-number is text");
	fails_with("f(p{phone-number:text}) := p.phone-number; f({name:\"Ann\"})", "missing argument field phone-number");
}

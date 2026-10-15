//! Card general-conversions (decision general-conversions): `A as B := body` defines a program's conversion of any
//! types, looked up before the built-in ones (src/lowering/types/conversions.rs)
use crate::is;

const PERSON: &str = "class Person{name:text}\n";

fn with_person(code: &str) -> String {
	format!("{PERSON}{code}")
}

#[test]
fn a_program_converts_its_class_to_text() {
	is!(&with_person("Person as text := it.name\np = Person{name:\"Ann\"}\np as text"), "Ann");
	is!(&with_person("Person as text := \"P:\" + it.name\nps = [Person{name:\"A\"}, Person{name:\"B\"}]\nps.map(p => p as text)[1]"), "P:B");
	is!(
		&with_person("Person as text := it.name\nshow(p: Person) := (p as text) + \"?\"\nshow(Person{name:\"Ann\"})"),
		"Ann?"
	);
}

#[test]
fn other_values_convert_the_built_in_way() {
	is!(&with_person("Person as text := it.name\n(3 as text) + \"!\""), "3!");
	is!(&with_person("Person as int := it.name.length\n(Person{name:\"Ann\"} as int) + (\"7\" as int)"), 10);
	is!(&with_person("Person as text := it.name\nx as text = \"a\"\nx"), "a");
}

#[test]
fn a_program_converts_into_its_class() {
	is!(&with_person("text as Person := Person{name: it}\nq = \"Bob\" as Person\nq.name"), "Bob");
}

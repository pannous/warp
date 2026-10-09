// notes/orm.md: a field of a table's element changed in place (`people#1.age = 5`, of a filter's row too) is written to
// its row as a variable's is (it was "people·at() gives a copy")
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(table: &str, rest: &str) -> String {
	format!("{PERSON}\npeople: [Person] = database.{table}\n{rest}")
}

#[test]
fn an_element_field_change_is_written_to_its_row() {
	eval(&program("people_element_write", "people.add(Person(\"Al\", 40))\npeople.add(Person(\"Bo\", 30))"));
	is!(&program("people_element_write", "people#1.age = 5\npeople#1.age"), 5);
	is!(&program("people_element_write", "people#1.age"), 5);
	eval(&program("people_element_write", "people#2.age += 2"));
	is!(&program("people_element_write", "people#2.age"), 32);
}

#[test]
fn a_filtered_row_field_change_is_written_to_its_row() {
	eval(&program("people_filter_write", "people.add(Person(\"Al\", 40))"));
	eval(&program("people_filter_write", "(people where it.name == \"Al\")#1.age = 6"));
	is!(&program("people_filter_write", "people#1.age"), 6);
}

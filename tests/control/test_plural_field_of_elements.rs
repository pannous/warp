// card plural-of: `names of people` is `name of people`, the field of each element named in the plural
use crate::is;

const PEOPLE: &str = "class P{name: text; age: int; city: text; address: text}\npeople: [P] = [P(\"Al\", 30, \"Rome\", \"a1\"), P(\"Bo\", 10, \"Oslo\", \"b2\")]";

fn program(rest: &str) -> String {
	format!("{PEOPLE}\n{rest}")
}

#[test]
fn a_plural_field_of_a_list_is_the_field_of_each_element() {
	is!(&program("str(names of people)"), "[\"Al\" \"Bo\"]");
	is!(&program("str(people.names)"), "[\"Al\" \"Bo\"]");
	is!(&program("str(ages of people)"), "[30 10]");
}

#[test]
fn english_plurals_of_fields() {
	is!(&program("str(cities of people)"), "[\"Rome\" \"Oslo\"]");
	is!(&program("str(addresses of people)"), "[\"a1\" \"b2\"]");
}

#[test]
fn a_plural_field_of_a_filtered_list() {
	is!(&program("str(names of people with age > 20)"), "[\"Al\"]");
}

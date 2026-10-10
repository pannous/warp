// `name of people with age > 20` (user, 2026-10-10): `field of list` is the field of each element, `with` filters a
// list of a class's instances like `where`; together [p.name for p in people if p.age > 20]
use crate::is;
use warp::wasm_emitter::eval;

const PEOPLE: &str = "class P{name: text; age: int}\npeople: [P] = [P(\"Al\", 30), P(\"Bo\", 10), P(\"Cy\", 40)]";

fn program(rest: &str) -> String {
	format!("{PEOPLE}\n{rest}")
}

#[test]
fn a_field_of_a_list_is_the_field_of_each_element() {
	is!(&program("str(name of people)"), "[\"Al\" \"Bo\" \"Cy\"]");
	is!(&program("count(name of people)"), 3);
	is!(&program("names = name of people\nnames#2"), "Bo");
	is!(&program("name of people#1"), "Al");
}

#[test]
fn with_filters_a_list_of_instances() {
	is!(&program("count(people with age > 20)"), 2);
	is!(&program("str(name of (people with age > 20))"), "[\"Al\" \"Cy\"]");
	is!(&program("str(name of people with age > 20)"), "[\"Al\" \"Cy\"]");
	is!(&program("grown = name of people with age > 20\ngrown#2"), "Cy");
	is!(&program("str(name of people where age > 20)"), "[\"Al\" \"Cy\"]");
}

#[cfg(feature = "native")]
#[test]
fn a_field_of_a_filtered_table_reads_only_its_rows() {
	let stored = |rest: &str| format!("class Person{{name: text; age: int}}\npeople: [Person] = database.people_field_of\n{rest}");
	eval(&stored("for i in 1 to 50 { people.add(Person(\"N\" + i, i)) }"));
	let before = warp::database::rows_read();
	is!(&stored("names = name of people with age > 48\nnames#1 + names#2"), "N49N50");
	assert_eq!(warp::database::rows_read(), before + 2, "name of a filtered table read other rows than its two");
}

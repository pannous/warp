// notes/orm.md Loading: a filter of an unloaded table is a query that reads only the rows it keeps, each kept as the
// one instance of its row; a loaded table's filter keeps its loaded instances
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(table: &str, rest: &str) -> String {
	format!("{PERSON}\npeople: [Person] = database.{table}\n{rest}")
}

#[cfg(feature = "native")]
#[test]
fn a_filter_reads_only_the_rows_it_keeps() {
	eval(&program("people_filtered_rows", "for i in 1 to 50 { people.add(Person(\"N\" + i, i)) }"));
	let before = warp::database::rows_read();
	is!(&program("people_filtered_rows", "count(people where it.age > 45)"), 5);
	assert_eq!(warp::database::rows_read(), before + 5, "the filter read other rows than its five");
}

// the row a filter gives is the instance people#i and the loaded list hold
#[test]
fn a_filtered_row_is_the_one_instance_of_its_row() {
	eval(&program("people_filtered_identity", "people.add(Person(\"Al\", 40))\npeople.add(Person(\"Bo\", 30))"));
	is!(&program("people_filtered_identity", "bo = (people where it.name == \"Bo\")#1\nbo.name = \"Bob\"\npeople#2.name"), "Bob");
	is!(&program("people_filtered_identity", "bo = people#2\nbo.age = 31\n(people where it.age == 31)#1.name"), "Bob");
	is!(&program("people_filtered_identity", "total = 0\nfor p in people { total += p.age }\nbo = (people where it.age > 30)#2\nbo.age = 32\ntotal = 0\nfor p in people { total += p.age }\ntotal"), 72);
	is!(&program("people_filtered_identity", "count(people where it.age > 0) + count(people)"), 4);
}

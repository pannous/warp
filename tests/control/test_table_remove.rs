#![cfg(feature = "native")]
// card table-remove: `people.remove(p)` deletes p's row, p keeping its fields without one (id 0); before and after the
// table loaded (found editing samples/orm.warp); `for p in people where … { … }` walks the filtered rows
use crate::is;
use warp::wasm_emitter::eval;

const SEED: &str = "people.add(Person(\"Ann\", 7))\npeople.add(Person(\"Bo\", 30))\npeople.add(Person(\"Cy\", 41))";

fn program(table: &str, rest: &str) -> String {
	format!("class Person{{name: text; age: int}}\npeople: [Person] = database.people_{table}\n{rest}")
}

#[test]
fn removing_a_row_before_the_table_loads() {
	eval(&program("removed_unloaded", SEED));
	is!(&program("removed_unloaded", "bo = people#2\npeople.remove(bo)\nbo.id"), 0);
	is!(&program("removed_unloaded", "count(people)"), 2);
	is!(&program("removed_unloaded", "people#2.name"), "Cy");
}

#[test]
fn removing_a_row_of_a_loaded_table() {
	eval(&program("removed_loaded", SEED));
	is!(&program("removed_loaded", "for gone in people where gone.age > 18 { people.remove(gone) }\ncount(people)"), 1);
	is!(&program("removed_loaded", "people.map(p => p.name)"), warp::texts(vec!["Ann"]));
}

// notes/orm.md Loading: a comprehension over a table with a filter, [p.name for p in people if p.age > 45], is the
// table's query, as people where it.age > 45 is: it reads only the rows it keeps
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(rest: &str) -> String {
	format!("{PERSON}\npeople: [Person] = database.people_comprehended\n{rest}")
}

#[cfg(feature = "native")]
#[test]
fn a_filtered_comprehension_over_a_table_reads_only_its_rows() {
	eval(&program("for i in 1 to 50 { people.add(Person(\"N\" + i, i)) }"));
	let before = warp::database::rows_read();
	is!(&program("names = [p.name for p in people if p.age > 47]\nnames#1 + names#3"), "N48N50");
	assert_eq!(warp::database::rows_read(), before + 3, "the comprehension read other rows than its three");
	is!(&program("least = 49\ncount([p.age * 2 for p in people if p.age >= least and p.name != \"N50\"])"), 1);
}

// card people-where-cond: [p.name for p in people where p.age > 45] is the table's query, as the comprehension with `if`
// is (test_table_comprehension): it reads only the rows it keeps
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(rest: &str) -> String {
	format!("{PERSON}\nstored people: [Person] in \"people_where_comprehended\"\n{rest}")
}

#[cfg(feature = "native")]
#[test]
fn a_comprehension_with_where_over_a_table_reads_only_its_rows() {
	eval(&program("for i in 1 to 50 { people.add(Person(\"N\" + i, i)) }"));
	let before = warp::host::database::rows_read();
	is!(&program("names = [p.name for p in people where p.age > 47]\nnames#1 + names#3"), "N48N50");
	assert_eq!(warp::host::database::rows_read(), before + 3, "the comprehension read other rows than its three");
}

#[test]
fn a_comprehension_with_where_over_a_list_filters() {
	is!("xs = [1, 5, 2, 7]; [x * 10 for x in xs where x > 2]", warp::ints(vec![50, 70]));
}

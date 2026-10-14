// card orm-paging (notes/orm.md Loading): a slice of an unloaded table, `people#(11…20)`, `people[10:20]`,
// `first 3 of people`, `people.first(3)`, `last 2 of people`, is a page query (LIMIT … OFFSET …) that reads only its rows,
// each the one instance of its row; a loaded table slices its list
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(table: &str, rest: &str) -> String {
	format!("{PERSON}\nstored people: [Person] in \"{table}\"\n{rest}")
}

fn fifty_people(table: &str) {
	eval(&program(table, "for i in 1 to 50 { people.add(Person(\"N\" + i, i)) }"));
}

#[cfg(feature = "native")]
#[test]
fn a_slice_of_a_table_reads_only_its_rows() {
	fifty_people("people_sliced");
	let read = |rest: &str, expected: i64, rows: usize| {
		let before = warp::host::database::rows_read();
		is!(&program("people_sliced", rest), expected);
		assert_eq!(warp::host::database::rows_read(), before + rows, "{rest} read other rows than its {rows}");
	};
	read("page = people#(11…20)\ncount(page) * 100 + page#1.age", 1011, 10);
	read("page = people#(11..20)\ncount(page)", 9, 9);
	read("page = people[10:20]\ncount(page) * 100 + page#10.age", 1020, 10);
	read("page = first 3 of people\npage#1.age + page#2.age + page#3.age", 6, 3);
	read("page = people.first(3)\ncount(page)", 3, 3);
	read("page = last 2 of people\npage#1.age + page#2.age", 99, 2);
	read("page = people#(49..60)\ncount(page)", 2, 2);
}

#[test]
fn a_sliced_row_is_the_one_instance_of_its_row() {
	fifty_people("people_sliced_identity");
	is!(&program("people_sliced_identity", "page = people#(2..3)\npage#1.name = \"Two\"\npeople#2.name"), "Two");
	is!(&program("people_sliced_identity", "people#2.name"), "Two");
	is!(&program("people_sliced_identity", "n = count(people)\npage = people#(48…50)\npage#3.age + n"), 100);
}

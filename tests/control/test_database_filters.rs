// card orm-filters (notes/orm.md step 2/3): bare field names in a filter (P223), and a filter of a table runs in its
// SQL query, the parts SQL has as plain SQL, any other warp expression as an SQLite application function
use crate::is;
#[cfg(feature = "native")]
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";
#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
const IS_PRIME: &str = "is_prime(n) := n > 1 and count([d for d in 2..n if n % d == 0]) == 0";

#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
fn table(name: &str, rest: &str) -> String {
	format!("{PERSON}\n{IS_PRIME}\npeople: [Person] = database.{name}\n{rest}")
}

#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
fn filled(name: &str) {
	eval(&table(name, "people.add(Person(\"Bo\", 7))\npeople.add(Person(\"Cy\", 8))\npeople.add(Person(\"anna\", 11))"));
}

/// P223 (user): `people where age > 20` is `people where it.age > 20` when Person has a field age
#[test]
fn a_bare_field_name_in_a_filter_is_the_field_of_each_element() {
	let people = format!("{PERSON}\npeople: [Person] = [Person(\"Bo\", 30), Person(\"Cy\", 10)]\n");
	is!(&format!("{people}count(people where age > 20)"), 1);
	is!(&format!("{people}limit = 20\ncount(people where age > limit and name != \"Cy\")"), 1);
	is!(&format!("{people}(people where name == \"Cy\")#1.age"), 10);
}

/// a variable of the same name wins, with a warning naming the field form
#[test]
fn a_variable_wins_over_a_field_of_the_same_name_with_a_warning() {
	warp::diagnostic::take_warnings();
	is!(&format!("{PERSON}\npeople: [Person] = [Person(\"Bo\", 30), Person(\"Cy\", 10)]\nname = \"Bo\"\ncount(people where it.name == name)"), 1);
	let warnings = warp::diagnostic::take_warnings();
	assert!(warnings.iter().any(|warning| warning.message.contains("it.name")), "{warnings:?}");
}

#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
#[test]
fn a_table_filter_runs_in_its_query() {
	filled("people_filtered");
	is!(&table("people_filtered", "count(people where age > 7)"), 2);
	is!(&table("people_filtered", "(people where age > 7 and name == \"anna\")#1.age"), 11);
	let lowered = warp::pipeline::lower(&table("people_filtered", "count(people where age > 7)")).expect("lowers").serialize();
	assert!(lowered.contains("\"select\""), "{lowered}");
}

/// any pure warp function in a filter: SQLite calls back into the program (sqlite3_create_function)
#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
#[test]
fn a_warp_function_in_a_table_filter_is_called_by_the_query() {
	filled("people_called");
	is!(&table("people_called", "count(people where is_prime(age))"), 2);
	is!(&table("people_called", "(people where name.reverse() == name)#1.name"), "anna");
	is!(&table("people_called", "count(people where is_prime(age) and age > 7)"), 1);
}

/// the rows a filter gives are the table's instances: a change through one is seen in the other
#[cfg(feature = "native")] // SQLite natively; the browser has no tables yet (notes/orm.md step 6)
#[test]
fn a_filtered_row_is_the_same_instance() {
	filled("people_same");
	is!(&table("people_same", "bo = (people where name == \"Bo\")#1\nbo.age += 1\npeople#1.age"), 8);
}

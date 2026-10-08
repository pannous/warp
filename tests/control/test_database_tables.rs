// card orm (notes/orm.md): a list of a class assigned from database.t is the table t, kept in SQLite natively
// (<program>.database.sqlite beside the program; code without a file keeps its tables while the thread runs)
use crate::is;
use warp::wasm_emitter::eval;

const PERSON: &str = "class Person{name: text; age: int}";

fn program(table: &str, rest: &str) -> String {
	format!("{PERSON}\npeople: [Person] = database.{table}\n{rest}")
}

#[test]
fn a_table_takes_rows_filters_and_writes_field_changes_through() {
	let run = |rest: &str| eval(&program("people_through", rest));
	run("people.add(Person(\"Bo\", 30))\npeople.add(Person(\"Cy\", 10))");
	is!(&program("people_through", "count(people where it.age > 20)"), 1);
	run("bo = (people where it.name == \"Bo\")#1\nbo.age += 1");
	is!(&program("people_through", "(people where it.name == \"Bo\")#1.age"), 31);
	is!(&program("people_through", "count(people)"), 2);
}

#[test]
fn a_row_knows_its_id() {
	eval(&program("people_ids", "people.add(Person(\"Al\", 40))"));
	is!(&program("people_ids", "people#1.id"), 1);
}

#[test]
fn a_field_added_to_the_class_adds_a_column() {
	eval("class Pet{name: text}\npets: [Pet] = database.pets_grown\npets.add(Pet(\"Rex\"))");
	is!("class Pet{name: text; legs: int = 4}\npets: [Pet] = database.pets_grown\ncount(pets)", 1);
	eval("class Pet{name: text; legs: int = 4}\npets: [Pet] = database.pets_grown\npets.add(Pet(\"Tweety\", 2))");
	is!("class Pet{name: text; legs: int = 4}\npets: [Pet] = database.pets_grown\npets#2.legs", 2);
}

// a removed field keeps its column (data is never dropped silently), loudly
#[test]
fn a_field_removed_from_the_class_keeps_its_column_with_a_warning() {
	eval("class Car{make: text; color: text}\ncars: [Car] = database.cars_shrunk\ncars.add(Car(\"VW\", \"red\"))");
	warp::diagnostic::take_runtime_warnings();
	is!("class Car{make: text}\ncars: [Car] = database.cars_shrunk\ncars#1.make", "VW");
	let warnings = warp::diagnostic::take_runtime_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("color")), "{warnings:?}");
}

#[cfg(feature = "native")]
#[test]
fn a_program_keeps_its_tables_in_a_file_beside_it() {
	let folder = std::path::Path::new("scratch/database_tables");
	std::fs::create_dir_all(folder).expect("scratch folder");
	let file = folder.join("app.database.sqlite");
	let _ = std::fs::remove_file(&file);
	let run = |rest: &str| warp::modules::with_program_file(&folder.join("app.warp"), || eval(&program("people", rest)));
	run("people.add(Person(\"Bo\", 30))");
	assert_eq!(run("people#1.name"), "Bo");
	assert!(file.exists());
}

// `stored people: [Person]` is the short form of `people: [Person] = database.people` (user, 2026-10-08)
#[test]
fn stored_registers_a_table_of_the_same_name() {
	eval("class Person{name: text; age: int}\nstored people: [Person]\npeople.add(Person(\"Di\", 50))");
	is!(&program("people", "people#1.name"), "Di");
}

// samples/orm.warp keeps its rows beside it and seeds them once, so each run gives the same
#[cfg(feature = "native")]
#[test]
fn the_orm_sample_runs_twice_alike() {
	is!("samples/orm.warp", "Bo has row 2");
	is!("samples/orm.warp", "Bo has row 2");
}

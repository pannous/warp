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

// a lossless type change converts the column in place, a lossy one is a loud error (notes/orm.md Migrations)
#[test]
fn a_field_changed_without_loss_converts_its_column() {
	eval("class Box{width: int}\nboxes: [Box] = database.boxes_widened\nboxes.add(Box(3))");
	is!("class Box{width: float}\nboxes: [Box] = database.boxes_widened\nboxes#1.width", 3.0);
	eval("class Tag{code: int}\ntags: [Tag] = database.tags_spelled\ntags.add(Tag(7))");
	is!("class Tag{code: text}\ntags: [Tag] = database.tags_spelled\ntags#1.code", "7");
}

#[test]
fn a_field_changed_with_loss_is_an_error_naming_both_types() {
	eval("class Bag{weight: float}\nbags: [Bag] = database.bags_narrowed\nbags.add(Bag(2.5))");
	crate::common::fails_with("class Bag{weight: int}\nbags: [Bag] = database.bags_narrowed\nbags#1.weight", "bags_narrowed.weight holds");
}

// a unit field's column keeps its unit (NUMERIC km) and the SI amount: km → m keeps the distances, km → kg is a loud
// error, plain numbers given a unit are read as that unit with a warning (card unit-fields); natively only so far (the
// playground's store refuses a unit change loudly, card browser-unit-migrations)
#[cfg(feature = "native")]
#[test]
fn a_unit_field_is_stored_and_migrates_within_its_quantity() {
	let runs = |class: &str, table: &str, rest: &str| format!("class Run{{{class}}}\nruns: [Run] = database.{table}\n{rest}");
	eval(&runs("distance: km", "runs_km", "runs.add(Run(5 km))\nruns.add(Run(1500 m))"));
	assert_eq!(eval(&runs("distance: km", "runs_km", "runs#2.distance")).serialize().trim(), "1.5km");
	assert_eq!(eval(&runs("distance: m", "runs_km", "runs#1.distance")).serialize().trim(), "5000m");
	crate::common::fails_with(&runs("distance: kg", "runs_km", "runs#1.distance"), "different quantities");
	crate::common::fails_with(&runs("distance: float", "runs_km", "runs#1.distance"), "the unit would be lost");
	eval(&runs("distance: int", "runs_plain", "runs.add(Run(3))"));
	warp::diagnostic::take_runtime_warnings();
	assert_eq!(eval(&runs("distance: km", "runs_plain", "runs#1.distance")).serialize().trim(), "3km");
	let warnings = warp::diagnostic::take_runtime_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("read as km")), "{warnings:?}");
}

#[test]
fn an_add_in_a_block_of_one_statement_inserts() {
	eval(&program("people_blocked", "if count(people) == 0 { people.add(Person(\"Hal\", 9)) }"));
	is!(&program("people_blocked", "people#1.id"), 1);
}

// the smart default (notes/orm.md Loading): registering a table loads no rows; count is SELECT COUNT(*), an add
// inserts without loading, and the rows load on the first read of the list
#[cfg(feature = "native")]
#[test]
fn a_table_loads_its_rows_only_when_read() {
	eval(&program("people_lazy", "people.add(Person(\"Jo\", 5))\npeople.add(Person(\"Ki\", 6))"));
	let before = warp::database::rows_read();
	is!(&program("people_lazy", "people.add(Person(\"Lu\", 7))\ncount(people)"), 3);
	assert_eq!(warp::database::rows_read(), before, "counting and adding loaded rows");
	is!(&program("people_lazy", "people#2.name"), "Ki");
	assert!(warp::database::rows_read() > before, "reading an element loads the rows");
}

// `people#i` reads that one row, kept so the loaded list holds the same instance
#[cfg(feature = "native")]
#[test]
fn an_element_of_a_table_loads_one_row() {
	eval(&program("people_indexed", "people.add(Person(\"Ny\", 5))\npeople.add(Person(\"Oz\", 6))\npeople.add(Person(\"Pi\", 7))"));
	let before = warp::database::rows_read();
	is!(&program("people_indexed", "people#2.name"), "Oz");
	assert_eq!(warp::database::rows_read(), before + 1, "people#2 read more than its row");
	is!(&program("people_indexed", "oz = people#2\noz.age = 60\nsum = 0\nfor p in people { sum += p.age }\nsum + people#2.age"), 72 + 60);
}

// iterating an unloaded table reads it in pages of 100 rows: a loop that breaks early reads only the pages it reached
#[cfg(feature = "native")]
#[test]
fn iterating_a_table_reads_it_in_pages() {
	eval(&program("people_paged", "for i in 1 to 250 { people.add(Person(\"N\" + i, i)) }"));
	let before = warp::database::rows_read();
	is!(&program("people_paged", "found = 0\nfor p in people { if p.age == 150 { found = p.age; break } }\nfound"), 150);
	assert_eq!(warp::database::rows_read(), before + 200, "a loop breaking at row 150 read other than two pages");
	is!(&program("people_paged", "sum = 0\nfor p in people { if p.age > 3 { continue }; sum += p.age }\nsum"), 6);
	is!(&program("people_paged", "for p in people { if p.age == 120 { p.name = \"Hundred20\" } }\npeople#120.name"), "Hundred20");
}

// an instance added before the rows load is the one the loaded list holds
#[test]
fn an_instance_added_before_loading_is_the_loaded_row() {
	is!(&program("people_known", "mo = Person(\"Mo\", 1)\npeople.add(mo)\nmo.age = 2\nfirst = people#1\nfirst.age = 3\nmo.age"), 3);
}

// `@was(old)` on a field renames its column, keeping the data; without it a rename reads as add + remove
#[test]
fn a_field_marked_was_renames_its_column() {
	eval("class Dog{nick: text}\ndogs: [Dog] = database.dogs_renamed\ndogs.add(Dog(\"Rex\"))");
	is!("class Dog{@was(nick) name: text}\ndogs: [Dog] = database.dogs_renamed\ndogs#1.name", "Rex");
	is!("class Dog{@was(nick) name: text}\ndogs: [Dog] = database.dogs_renamed\ndogs#1.name", "Rex");
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

// `save p` writes p's row and is p; field changes are written through already (user default, 2026-10-08)
#[test]
fn save_writes_the_row_and_is_the_instance() {
	let saved = eval(&program("people_saved", "people.add(Person(\"Eve\", 20))\neve = people#1\neve.age = 21\nsave eve"));
	assert!(saved.serialize().contains("Eve"), "{saved:?}");
	is!(&program("people_saved", "people#1.age"), 21);
	crate::common::fails_with(&program("people_unsaved", "save Person(\"Fay\", 3)"), "add it to people first");
}

// as any expression: the standalone build of a program ending with `save bo` prints it, `print(save bo)` (card orm-standalone)
#[test]
fn save_is_an_expression() {
	is!(&program("people_saved_inline", "people.add(Person(\"Gus\", 60))\nprint(save people#1)\n(save people#1).age"), 60);
}

// samples/orm.warp keeps its rows beside it (in the browser in IndexedDB) and seeds them once; each run makes Bo (row 2)
// a year older and saves him
#[test]
fn the_orm_sample_runs_twice_on_the_same_rows() {
	let runs = [eval("samples/orm.warp").serialize(), eval("samples/orm.warp").serialize()];
	for run in &runs {
		assert!(run.contains("Bo") && run.contains("id:2"), "{run}");
	}
	assert_ne!(runs[0], runs[1], "the second run reads the age the first one wrote");
}

// card todo-app robustness: `people.remove(p)` deletes p's row, loaded or not, in a function too
#[test]
fn remove_deletes_the_row() {
	let run = |rest: &str| eval(&program("people_removed", rest));
	run("people.add(Person(\"Al\", 1))\npeople.add(Person(\"Bo\", 2))\npeople.add(Person(\"Cy\", 3))");
	is!(&program("people_removed", "people.remove((people where it.name == \"Bo\")#1)\ncount(people)"), 2);
	is!(&program("people_removed", "count(people)"), 2);
	is!(&program("people_removed", "gone(name) := { people.remove((people where it.name == name)#1) }\ngone(\"Al\").age"), 1);
	is!(&program("people_removed", "people#1.name"), "Cy");
	is!(&program("people_removed", "people.remove(people#1)\npeople.add(Person(\"Di\", 4))\npeople#1.name"), "Di");
}

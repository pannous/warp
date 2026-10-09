// card orm-sample-lock: programs sharing a database file wait for each other's writes (sqlite3_busy_timeout), and
// cargo's test runs give each test thread its own copy of a sample's tables (WARP_TEST_TABLES, .cargo/config.toml)
#[cfg(feature = "native")]
use warp::wasm_emitter::eval;

#[cfg(feature = "native")]
const WRITERS: usize = 6;

#[cfg(feature = "native")]
#[test]
fn programs_writing_one_database_at_once_wait_for_each_other() {
	let folder = std::path::Path::new("scratch/database_busy");
	std::fs::create_dir_all(folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = "class Person{name: text; age: int}\nstored people: [Person]\nfor i in 1..20 { people.add(Person(\"Al\", i)) }\ncount(people)";
	// unnamed threads, as a program's own tasks: they share the one file beside the program
	let writers: Vec<_> = (0..WRITERS).map(|_| {
		let file = folder.join("app.warp");
		std::thread::spawn(move || warp::modules::with_program_file(&file, || eval(program)).serialize())
	}).collect();
	for writer in writers {
		let result = writer.join().expect("writer thread");
		assert!(!result.contains("locked"), "{result}");
	}
}

#[cfg(feature = "native")]
#[test]
fn a_test_thread_keeps_a_samples_tables_apart() {
	let folder = std::env::var("WARP_TEST_TABLES").expect("WARP_TEST_TABLES from .cargo/config.toml");
	let thread = std::thread::current().name().expect("libtest names its threads").replace("::", ".");
	eval("samples/orm.warp");
	let file = std::path::Path::new(&folder).join(thread).join("orm.database.sqlite");
	assert!(file.exists(), "{}", file.display());
}

// card web-stores: `stored theme = "dark"` is a persisted signal: the value an earlier run kept under its name, else the
// default, and each change is kept again (lowering/stored_values.rs). Natively in <program>.stored.json next to the
// program; code without a file keeps its values while the process runs (per thread: each test has its own names)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_stored_value_starts_with_its_default() {
	is!("stored theme_first = \"dark\"; theme_first", "dark");
	is!("stored count_first = 1; count_first + 1", 2);
}

#[test]
fn a_change_of_a_stored_value_is_kept_for_the_next_run() {
	eval("stored visits_kept = 0; visits_kept += 1; visits_kept");
	eval("stored visits_kept = 0; visits_kept += 1; visits_kept");
	is!("stored visits_kept = 0; visits_kept", 2);
	eval("stored theme_kept = \"dark\"; theme_kept = \"light\"");
	is!("stored theme_kept = \"dark\"; theme_kept", "light");
}

// the playground keeps them in the page's localStorage instead (host.js STD_ADAPTERS.store)
#[cfg(feature = "native")]
#[test]
fn a_program_keeps_its_stored_values_in_a_file_beside_it() {
	let folder = std::path::Path::new("scratch/stored_signals");
	std::fs::create_dir_all(folder).expect("scratch folder");
	let program = folder.join("app.warp");
	let store = folder.join("app.stored.json");
	let _ = std::fs::remove_file(&store);
	let run = || warp::modules::with_program_file(&program, || eval("stored launches = 0; launches += 1; launches"));
	run();
	run();
	let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&store).expect("the store file")).expect("json");
	assert_eq!(kept["launches"], 2);
}

// a soft keyword (P165): a local may be named stored
#[test]
fn a_local_named_stored_keeps_the_word() {
	is!("def f() { stored = 3; stored + 1 }; f()", 4);
}

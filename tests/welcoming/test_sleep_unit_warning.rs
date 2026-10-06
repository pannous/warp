// User issue #17: `sleep(1)` warns that sleep needs a unit (second or millisecond); `sleep 1 s` does not
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings().iter().map(|warning| warning.to_string()).collect()
}

#[test]
fn sleep_without_a_unit_warns() {
	let warnings = warnings_of("sleep(1)");
	assert!(warnings.iter().any(|warning| warning.contains("sleep needs a unit")), "{warnings:?}");
	assert!(warnings_of("sleep 1 ms").iter().all(|warning| !warning.contains("unit")));
	assert!(warnings_of("sleep(1 second)").iter().all(|warning| !warning.contains("unit")));
}

// `sleep(300 ms); 1` is two statements: the value after them is no duration of sleep
#[test]
fn a_statement_after_sleep_is_no_duration() {
	assert!(warnings_of("sleep(3 ms); 1").iter().all(|warning| !warning.contains("unit")));
	assert!(warnings_of("f() := { sleep(3 ms); 1 }; f()").iter().all(|warning| !warning.contains("unit")));
}

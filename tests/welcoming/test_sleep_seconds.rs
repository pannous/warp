// decision sleep-seconds: a bare number of sleep counts seconds, as in sh and Python, with the warning that asks for the
// unit; `use relaxed` takes it silently
use crate::is;
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

#[test]
fn a_bare_sleep_number_counts_seconds() {
	is!("start = clock(); sleep 0.2; clock() - start >= 200", true);
	is!("start = clock(); sleep(0.05); clock() - start >= 50", true);
	is!("start = clock(); sleep 20ms; clock() - start < 1000", true);
	is!("nap(t) := { sleep(t) }; start = clock(); nap(0.2); clock() - start >= 200", true);
	is!("nap(t) := { sleep(t) }; start = clock(); nap(1/20); clock() - start < 1000", true);
}

#[test]
fn the_bare_sleep_warning_suggests_seconds() {
	take_warnings();
	eval("sleep 0.01");
	let warnings: Vec<String> = take_warnings().iter().map(|warning| warning.to_string()).collect();
	assert!(warnings.iter().any(|warning| warning.contains("sleep 0.01s")), "{warnings:?}");
	eval("use relaxed\nsleep 0.01");
	assert!(take_warnings().is_empty());
}

// Runs stop at a time limit of computing instead of counting fuel steps, which made calls about four times slower
// (decision fuel-metering); `use fuel` and `warp fuel <steps>` count steps on request
use std::time::Duration;
use warp::node::Node;
use warp::util::{with_time_limit, TIME_LIMIT_VARIABLE};
use warp::wasm_emitter::eval;

const SHORT_LIMIT: Duration = Duration::from_millis(200);

#[test]
fn a_runaway_stops_at_the_time_limit() {
	match with_time_limit(SHORT_LIMIT, || eval("i = 0; while 1 { i = i + 1 }")) {
		Node::Error(message) => {
			let message = format!("{message}");
			assert!(message.contains("out of time") && message.contains(TIME_LIMIT_VARIABLE), "{message}");
		}
		other => panic!("expected out of time, got {other:?}"),
	}
}

#[test]
fn sleeping_is_no_computing() {
	assert_eq!(with_time_limit(SHORT_LIMIT, || eval("sleep 500ms; 7")), eval("7"));
}

#[test]
fn use_fuel_marks_the_module() {
	let module = warp::pipeline::compile("use fuel\nx = 3\nx + 4").unwrap_or_else(|failure| panic!("{failure:?}"));
	assert!(warp::host::host_parts::has_section(&module.bytes, warp::host::FUEL_SECTION));
	let plain = warp::pipeline::compile("x = 3\nx + 4").unwrap_or_else(|failure| panic!("{failure:?}"));
	assert!(!warp::host::host_parts::has_section(&plain.bytes, warp::host::FUEL_SECTION));
}

#[test]
fn a_use_fuel_program_runs() {
	assert_eq!(eval("use fuel\nn = 3\nn + 4"), eval("7"));
}

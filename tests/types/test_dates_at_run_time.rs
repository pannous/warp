//! Dates at run time (card runtime-dates, notes/dates_at_run_time.md): `now` reads the host's clock when the program
//! runs, not when it compiles; an instant is a $Node of Kind::Time holding nanoseconds since 1970 UTC
use crate::is;
use warp::node::Node;
use warp::time::Time;
use warp::wasm_emitter::eval;

/// Instants this close to the test's clock count as now
const NOW_TOLERANCE_NANOS: i128 = 5_000_000_000;

fn nanos_since_epoch() -> i128 {
	std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("after 1970").as_nanos() as i128
}

fn instant_of(node: &Node) -> i128 {
	match node.drop_meta() {
		Node::Data(data) => match data.downcast_ref::<Time>() {
			Some(Time::Instant(nanos)) => *nanos,
			_ => panic!("expected an instant, got {node:?}"),
		},
		Node::Text(text) => instant_of(&Node::data(warp::time::calendar::parse_literal(text.trim()).expect("an RFC 3339 instant"))),
		_ => panic!("expected an instant, got {node:?}"),
	}
}

fn assert_now(node: &Node) {
	let distance = (instant_of(node) - nanos_since_epoch()).abs();
	assert!(distance < NOW_TOLERANCE_NANOS, "{node:?} is {distance} ns away from now");
}

#[test]
#[cfg(feature = "native")] // the browser build runs modules in the page
fn test_now_reads_the_clock_when_the_program_runs() {
	let module = warp::pipeline::compile("now").expect("now needs a module: it is no constant");
	let first = warp::wasm_reader::read_bytes_with_host(&module.bytes).expect("runs");
	std::thread::sleep(std::time::Duration::from_millis(20));
	let second = warp::wasm_reader::read_bytes_with_host(&module.bytes).expect("runs again");
	assert_now(&first);
	assert!(instant_of(&second) > instant_of(&first), "{second:?} is not after {first:?}");
}

#[test]
fn test_now_reads_back_as_an_instant() {
	assert_now(&eval("now"));
	assert_now(&eval("t = now; t"));
}

#[test]
fn test_an_instant_shows_as_rfc_3339() {
	let shown = eval("str(now)");
	assert!(matches!(&shown, Node::Text(text) if text.ends_with('Z') && text.contains('T')), "{shown:?}");
	assert_now(&shown);
	let joined = eval("\"at \" + now");
	assert!(matches!(&joined, Node::Text(text) if text.starts_with("at 20")), "{joined:?}");
}

#[test]
#[cfg(feature = "native")]
fn test_print_now_prints_the_time() {
	let printed = crate::common::printed("print now");
	assert_now(&Node::Text(printed.lines().next().expect("a line").to_string()));
}

#[test]
fn test_instants_compare_at_run_time() {
	is!("a = now; b = now; b >= a", true);
	is!("a = now; b = now; a <= b and not (b < a)", true);
	is!("t = now; u = t; t == u", true);
}

/// An instant's wall clock is the environment's time zone (user, 2026-10-10, replacing "instant has no hour")
#[test]
fn test_an_instant_has_the_local_wall_clock() {
	is!("t = now; t.hour >= 0 and t.hour < 24 and t.year >= 2026", true);
}

// Page events (notes/signals.md phase 7): `on click {…}` and `on key {…}` are handlers the browser playground calls
// when the page sees the event, so they exist without a `raise` in the program, exported with a wrapper that takes
// the event in a list (on·click·node), as a task of values is started
use crate::is;

fn exports_of(code: &str) -> Vec<String> {
	let module = warp::pipeline::compile(code).unwrap_or_else(|value| panic!("{code}: {value:?}"));
	wasmparser::Parser::new(0).parse_all(&module.bytes).filter_map(Result::ok).flat_map(|payload| match payload {
		wasmparser::Payload::ExportSection(exports) => exports.into_iter().filter_map(Result::ok).map(|export| export.name.to_string()).collect(),
		_ => vec![],
	}).collect()
}

#[test]
fn a_page_event_handler_is_exported_without_a_raise() {
	is!("count = 0; on click { count += 1; count }; count", 0);
	let exports = exports_of("count = 0; on click { count += 1; count }; count");
	assert!(exports.iter().any(|name| name == "on·click·node"), "{exports:?}");
	assert!(exports_of("on key { print event.key }; 1").iter().any(|name| name == "on·key·node"));
}

#[test]
fn a_raised_page_event_runs_its_handler_as_any_event() {
	is!("count = 0; on click { count += 1 }; emit click; emit click; count", 2);
}

#[test]
fn a_page_event_handler_in_a_program_with_tasks_has_one_wrapper() {
	is!("count = 0; on click { count += 1 }; twice(x) := x * 2; job = go twice(21); await job", 42);
}

// the output binding: a program with page events whose last line is a name shows it again after each handler, read
// anew by page·value (a variable as it is then, a := getter recomputed)
#[test]
fn the_last_name_of_a_program_with_page_events_is_its_output_binding() {
	assert!(exports_of("count = 0; on click { count += 1 }; count").iter().any(|name| name == "page·value"));
	assert!(exports_of("price = 3; total := price * 2; on click { price += 1 }; total").iter().any(|name| name == "page·value"));
	assert!(!exports_of("on click { print 1 }; 1 + 2").iter().any(|name| name == "page·value"));
	is!("price = 3; total := price * 2; on click { price += 1 }; emit click; total", 8);
}

// the page keeps a program with timers or channel listeners running too, so its last name is shown anew after each tick
#[test]
fn the_last_name_of_a_program_with_timers_is_its_output_binding() {
	assert!(exports_of("ticks = 0; on every 1 second { ticks += 1 }; ticks").iter().any(|name| name == "page·value"));
	assert!(exports_of("n = 0; on message from \"chat\" { n += 1 }; n").iter().any(|name| name == "page·value"));
	assert!(!exports_of("ticks = 0; on every 1 second { ticks += 1 }; ticks + 1").iter().any(|name| name == "page·value"));
	assert!(!exports_of("ticks = 0; ticks").iter().any(|name| name == "page·value"));
}

// a handler that never reads `event` takes no parameter, so the page can call it (an unread one would be typed Int)
#[test]
fn a_handler_without_event_runs_through_its_wrapper() {
	is!("price = 3; on key { price = 10 }; emit key{key:\"a\"}; price", 10);
	is!("n = 0; on alarm { n += 1 }; emit alarm{level:3}; emit alarm; n", 2);
}

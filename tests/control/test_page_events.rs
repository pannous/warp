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
	is!("count = 0; on click { count += 1 }; raise click; raise click; count", 2);
}

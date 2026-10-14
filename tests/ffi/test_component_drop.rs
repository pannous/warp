//! `c.drop()` of a component's resource handle (card component-drop): the resource's destructor runs in the component
//! now, not when the process ends; a dropped handle is refused afterwards (tests/fixtures/components/rust_demo:
//! `live()` counts the counters not yet dropped)
use crate::common::fails_with;
use crate::is;

const RUST_DEMO: &str = "use wasm \"tests/fixtures/components/rust_demo.wasm\" as r\n";

fn using(code: &str) -> String {
	format!("{RUST_DEMO}{code}")
}

#[test]
fn dropping_a_handle_runs_the_destructor() {
	is!(&using("a = r.counter(1)\nb = r.counter(2)\nbefore = r.live()\na.drop()\nbefore - r.live()"), 1);
	is!(&using("a = r.counter(1)\nb = r.counter(2)\na.drop()\nb.value()"), 2);
}

#[test]
fn a_dropped_handle_is_refused() {
	fails_with(&using("c = r.counter(1)\nc.drop()\nc.value()"), "was dropped");
	fails_with(&using("c = r.counter(1)\nc.drop()\nc.drop()"), "was dropped");
	fails_with(&using("c = r.counter(1)\nc.drop()\nr.total([c])"), "was dropped");
}

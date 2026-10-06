//! `use wasm "lib.wasm"` in both hosts: natively wasmtime runs the component (src/components.rs), in the browser the
//! script jco transpiled from it (web/playground/components.js, build.sh components); same values, same handles
use crate::common::fails_with;
use warp::ints;
use crate::is;

const COMPONENT: &str = "tests/fixtures/components/rust_demo.wasm";

fn using(code: &str) -> String {
	format!("use wasm \"{COMPONENT}\" as r; {code}")
}

#[test]
fn test_a_component_is_called_in_every_host() {
	is!(&using("r.fib(50)"), 12586269025i64);
	is!(&using("r.words(\"a bc d\")"), warp::node::Node::List(vec!["a".into(), "bc".into(), "d".into()], warp::node::Bracket::Square, warp::node::Separator::Space));
	is!(&using("s = r.stats_of(\"hello big world\"); [s.words, s.letters]"), ints(vec![3, 13]));
	is!(&using("r.sides(\"square\")"), 4);
	is!(&using("r.checked_sqrt(16.0)"), 4.0);
}

#[test]
fn test_resources_of_a_component_are_handles_in_every_host() {
	is!(&using("c = r.counter(5); c.increment(2); c.value()"), 7);
	is!(&using("a = r.Counter(2); b = r.counter(3); m = r.merged(a, b); m.value()"), 5);
	is!(&using("a = r.counter(2); b = r.counter(3); r.total([a, b])"), 5);
}

#[test]
fn test_component_failures_are_loud_in_every_host() {
	fails_with(&using("r.checked_sqrt(-1.0)"), "no real square root of -1");
	fails_with(&using("r.nope(1)"), "exports no function nope; it exports fib");
	fails_with(&using("r.fib(1, 2)"), "fib takes 1 arguments");
	fails_with(&using("c = r.counter(1); c.nope()"), "a counter has no method nope");
	fails_with("use wasm \"no/such.wasm\" as lib; lib.f(1)", "cannot load the component");
}

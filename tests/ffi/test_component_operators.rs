//! `a + b` of a component's resource handles calls the resource's method named like the operator, `a.add(b)`
//! (card component-operators): add, sub, mul, div, mod, pow, lt, gt, le, ge, eq, ne; the component's plain results stay
//! warp's arithmetic (tests/fixtures/components/rust_demo: counter's add and lt)
use crate::common::fails_with;
use crate::is;

const RUST_DEMO: &str = "use wasm \"tests/fixtures/components/rust_demo.wasm\" as r\n";

fn using(code: &str) -> String {
	format!("{RUST_DEMO}a = r.counter(2)\nb = r.counter(3)\n{code}")
}

#[test]
fn operators_of_handles_call_their_methods() {
	is!(&using("(a + b).value()"), 5);
	is!(&using("c = a + b\nd = c + a\nd.value()"), 7);
	is!(&using("if a < b { 1 } else { 0 }"), 1);
	is!(&using("if b < a { 1 } else { 0 }"), 0);
}

#[test]
fn plain_results_keep_warps_operators() {
	is!(&using("n = r.fib(10)\nn * 2 + a.value()"), 112);
}

#[test]
fn an_operator_without_its_method_is_loud() {
	fails_with(&using("a * b"), "a counter has no method mul");
}

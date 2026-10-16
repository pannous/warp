//! card callable: a `callable` parameter takes a function or a block, and calling it calls the function or runs the
//! block; invocable and runnable are its aliases (hinted), `x is callable` tests for either
use crate::is;

const RUN: &str = "run(f: callable) := f(3)\n";

#[test]
fn a_callable_parameter_takes_a_function() {
	is!(&format!("square(x) := x * x\n{RUN}run(square)"), 9);
	is!(&format!("{RUN}run(x => x * 2)"), 6);
	is!(&format!("g = x => x + 1\n{RUN}run(g)"), 4);
	is!("hi() := 7\nrun(f: callable) := f()\nrun(hi)", 7);
}

#[test]
fn a_callable_parameter_takes_a_block() {
	is!("run(f: callable) := f()\nrun({ 1 + 2 })", 3);
	is!("n = 0\ntwice(f: callable) := { f(); f() }\ntwice({ n += 1 })\nn", 2);
	is!("run(f: invocable) := f()\nrun({ 1 + 2 })", 3);
	is!("run(f: runnable) := f()\nrun({ 1 + 2 })", 3);
}

#[test]
fn callable_is_a_type() {
	is!("square(x) := x * x\nsquare is callable", true);
	is!("3 is callable", false);
	is!("\"a\" is callable", false);
}

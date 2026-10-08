//! Card effect-handlers step 2 (notes/effect_handlers.md): an emitted event is a named effect. `emit ask` in f adds
//! `ask` to f's effects; a call inside `on ask {…} in {…}` does not pass it on; `! Pure` reports it, `! ask` allows it
use crate::common::fails_with;
use crate::is;
use warp::Node;

fn symbol(name: &str) -> Node {
	Node::Symbol(name.into())
}

#[test]
fn an_emitted_event_is_a_named_effect() {
	is!("compute() := emit ask\neffects of compute", symbol("ask"));
	is!("compute() := emit ask\ntwice() := compute() + compute()\neffects of twice", symbol("ask"));
	is!("compute() := { puts \"hi\"; emit ask }\neffects of compute", warp::wasp_parser::parse("(IO ask)"));
}

#[test]
fn a_block_handler_handles_its_event() {
	is!("compute() := emit ask\nrun() := on ask { 42 } in { compute() }\neffects of run", symbol("Pure"));
	is!("compute() := { emit ask; emit tell }\nrun() := on ask { 42 } in { compute() }\neffects of run", symbol("tell"));
}

#[test]
fn a_declared_effect_set_reports_an_unhandled_event() {
	fails_with("compute() := { emit ask; 1 } ! Pure\n3", "compute is declared ! Pure but performs ask");
	is!("compute() := { emit ask; 1 } ! ask\n3", 3);
	is!("compute() := emit ask\nrun() := on ask { 42 } in { compute() } ! Pure\nrun()", 42);
}

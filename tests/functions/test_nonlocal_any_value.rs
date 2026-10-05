//! Card nonlocal-cells: a nested function changes a nonlocal of any value (a cell holds any Node: wasm_emitter/cells.rs)
use warp::is;

#[test]
fn test_inner_changes_a_nonlocal_text() {
	is!("def outer(){ s=\"a\"; def inner(){ nonlocal s; s = s + \"!\" }; inner(); inner(); s }; outer()", "a!!");
}

#[test]
fn test_inner_changes_a_nonlocal_list() {
	is!("def outer(){ xs=[1]; def inner(){ nonlocal xs; xs = xs + [5] }; inner(); xs#2 }; outer()", 5);
	is!("def outer(){ xs=[1]; def inner(){ nonlocal xs; xs = xs + [2] }; inner(); inner(); count xs }; outer()", 3);
}

#[test]
fn test_an_escaping_closure_keeps_a_text_cell() {
	is!("def make(){ log=\"\"; def add(){ nonlocal log; log = log + \"x\"; log }; add }; a = make(); b = make(); a(); b(); a()", "xx");
}

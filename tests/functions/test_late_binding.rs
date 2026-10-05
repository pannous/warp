// Charged bodies read their free variables like Python functions (wiki/charged.md section 3, released 2026-10-05,
// package 2): at call time; a free variable that changes after the definition needs `global y` in the reading function,
// else the change is a compile error where a later call sees it. `name := expr` without parameters waits for P71.
use warp::*;

use crate::common;

const DECLARE_GLOBAL: &str = "z reads y (line 1): declare `global y` in z to read its current value, or pass y as a parameter";

#[test]
fn a_free_variable_changed_after_the_definition_is_an_error_at_the_change() {
	common::fails_with("y=3; def z(): y*y; y=4; z()", DECLARE_GLOBAL);
	common::fails_with("y=3\ndef z(): y*y\ny=4\nz()", "at 3:");
	common::fails_with("y=3; def z(){ y*y }; y+=1; z()", "fix: global y");
	common::fails_with("y=1; def z(): y; for i in 1..3 { y = i }; z()", "z reads y");
}

#[test]
fn declaring_global_reads_the_current_value() {
	is!("y=3; def z(){ global y; y*y }; y=4; z()", 16);
	is!("y=3\ndef z():\n  global y\n  y*y\ny=4\nz()", 16);
	is!("global y=3; def z(): y*y; y=4; z()", 16);
}

#[test]
fn a_free_variable_that_never_changes_needs_no_declaration() {
	is!("y=3; def z(): y*y; z()", 9);
	is!("y=3; y=4; def z(): y*y; z()", 16);
	is!("k=3; def f(x){x+k}; f(1)", 4);
}

#[test]
fn a_variable_bound_after_the_definition_is_read_at_call_time() {
	is!("def f(x){x+k}; k=3; f(1)", 4);
}

#[test]
fn in_place_mutation_counts_as_a_change() {
	common::fails_with("xs=[1,2]; def n(): count(xs); xs.add(3); n()", "n reads xs");
	is!("xs=[1,2]; def n(){ global xs; count(xs) }; xs.add(3); n()", 3);
}

#[test]
fn a_change_no_later_call_can_observe_is_fine() {
	is!("x=1; inc:={x+1}; x=inc(); x", 2);
	is!("y=3; def z(): y*y; a=z(); y=4; a", 9);
}

#[test]
fn loop_variables_are_captured_per_iteration() {
	is!("fs = []; for i in 0..3 { fs.add(x => x + i) }; fs#1(0) + 10*fs#2(0) + 100*fs#3(0)", 210);
}

#[test]
#[ignore = "soon"] // nested function definitions are not supported yet (`def outer(){ def inner(){…} }`)
fn nonlocal_reads_the_enclosing_functions_current_value() {
	is!("def outer(){ y=1; def inner(){ nonlocal y; y }; y=2; inner() }; outer()", 2);
}

fn hints_of(code: &str) -> Vec<(String, String)> {
	warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code)).1.into_iter().map(|hint| (hint.canonical, hint.reason)).collect()
}

#[test]
fn a_def_without_parameters_over_constants_gets_a_note() {
	is!("def area(): 3*4; area()+1", 13);
	let hints = hints_of("def area(): 3*4; area()");
	assert!(hints.iter().any(|(canonical, reason)| canonical == "area = 3*4" && reason == "area never changes"), "{hints:?}");
	assert!(hints_of("def t(): clock(); t()").iter().all(|(_, reason)| !reason.contains("never changes")));
	assert!(hints_of("def double(x): x*2; double(2)").iter().all(|(_, reason)| !reason.contains("never changes")));
}

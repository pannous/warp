// D7 (user 2026-10-03, "By value + educate"): a block captures outer variables by value, so a block that assigns an
// outer variable leaves it unchanged, and the compiler hints `global x` or returning the value.
use crate::is;
use warp::normalize::capture_hints;

fn hints_of(code: &str) -> Vec<String> {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.into_iter().map(|hint| hint.canonical).collect()
}

#[test]
fn a_block_that_assigns_an_outer_variable_leaves_it_unchanged() {
	is!("x=1; inc:={x=x+1}; do inc; x", 1);
	is!("x=1; inc:={x=x+1}; inc(); x", 1);
	is!("x=1; inc:={x+=1}; do inc; do inc; x", 1);
	is!("x=1; inc:={x=x+1; 0}; inc(); x", 1);
}

#[test]
fn a_block_that_assigns_an_outer_variable_hints_global_or_return() {
	let hints = hints_of("x=1; inc:={x=x+1}; do inc; x");
	assert!(hints.iter().any(|hint| hint.contains("global x")), "{hints:?}");
	assert!(hints.iter().any(|hint| hint == "x = inc()"), "{hints:?}");
}

#[test]
fn the_declared_and_returned_forms_change_it() {
	is!("global x=1; inc:={x=x+1}; do inc; x", 2);
	is!("x=1; inc:={x+1}; x=inc(); x", 2);
}

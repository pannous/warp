//! Card g-rQ-U: a nested function is reachable beyond its enclosing body's own calls (a sibling calls it, it is passed
//! around as a value) and `nonlocal y` still reads y as it is at that call; a nested function reads main-level variables
use warp::is;

#[test]
fn test_a_sibling_calls_a_nested_function() {
	is!("def outer(){ y=1; def inner(){ nonlocal y; y }; def sibling(){ inner() }; y=7; sibling() }; outer()", 7);
}

#[test]
fn test_a_nested_function_passed_around_reads_the_current_value() {
	is!("def apply(f){ f() }; def outer(){ y=1; def inner(){ nonlocal y; y }; y=9; apply(inner) }; outer()", 9);
	is!("def outer(){ y=1; def inner(){ nonlocal y; y }; g = function inner; y=4; g() }; outer()", 4);
}

#[test]
fn test_a_nested_function_reads_a_main_level_variable() {
	is!("k=5; def outer(){ def inner(){ k }; inner() }; outer()", 5);
}

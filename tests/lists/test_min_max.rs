//! Builtins `min` and `max`.

use crate::common::fails_with;
use crate::is;

#[test]
fn test_min_max_of_ints() {
	is!("min(1,2)", 1);
	is!("max(1,2)", 2);
	is!("min(3,1,2)", 1);
	is!("max(3,1,2)", 3);
}

#[test]
fn test_min_max_of_floats_and_mixed() {
	is!("min(1.5,2.5)", 1.5);
	is!("max(1.5,2)", 2);
}

#[test]
fn test_min_max_of_variables_and_expressions() {
	is!("x=5; min(x,3)", 3);
	is!("max(1+1,3)", 3);
	is!("1+max(1,2)", 3);
	is!("max(1,min(5,2))", 2);
}

#[test]
fn test_user_definition_wins_over_the_builtin() {
	is!("min(a,b):=a*b; min(2,3)", 6);
}

#[test]
fn test_min_max_arguments_must_be_side_effect_free() {
	is!("f(x):=x; min(f(1),2)", 1);
}

#[test]
fn test_min_needs_two_arguments() {
	fails_with("min(1)", "min takes at least 2");
}

/// Each comparison names the best so far twice: unbound, a list of n calls was 2ⁿ nodes (154 GB for 35, 2026-10-09)
#[test]
fn test_many_extremum_arguments_compile_linearly() {
	let calls: Vec<String> = (1..=40).map(|i| format!("abs({i} - 0.5)")).collect();
	is!(&format!("max([{}])", calls.join(", ")), 39.5);
	is!(&format!("min({})", calls.join(", ")), 0.5);
}

#[test]
fn test_a_float_max_accumulates_in_a_loop() {
	// card max-typed (samples/gpu_visualizer.warp): the lowered max binds l[b] to a temporary inside `r += (…)`
	is!("l=[0.0 as float for c in 0..2]; r=0.0; for b in 0..2 { x = 0.5; r += max(0.0, x - l[b]) }; r", 1.0);
	is!("r=0.0; r += (t = 0.5 as float; t); r", 0.5);
}

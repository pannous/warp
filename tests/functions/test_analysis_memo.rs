//! P91: one analysis per program state. The same tree is analysed once (analysis_memo.rs); an analysis that says
//! something (a warning, a hint) is not remembered, so a repeat says it again
use std::cell::Cell;
use warp::analysis_memo::analysed;
use warp::context::Context;
use warp::diagnostic::{report, take_warnings, Diagnostic};
use warp::warp_parser::parse;

#[test]
fn test_the_same_tree_is_analysed_once() {
	let program = parse("square(x) := x*x; square(memo_probe_one)");
	let runs = Cell::new(0);
	for _ in 0..3 {
		analysed(&mut Context::new(), &program, |context, node| {
			runs.set(runs.get() + 1);
			warp::analyzer::extract_user_functions(context, node);
		});
	}
	assert_eq!(runs.get(), 1);
}

#[test]
fn test_an_analysis_that_says_something_runs_again() {
	let program = parse("memo_probe_two(x) := x");
	let runs = Cell::new(0);
	take_warnings();
	for _ in 0..2 {
		analysed(&mut Context::new(), &program, |_, node| {
			runs.set(runs.get() + 1);
			report(&[Diagnostic::at(node, "memo probe")]).unwrap();
		});
	}
	assert_eq!(runs.get(), 2);
	assert_eq!(take_warnings().len(), 2);
}

#[test]
fn test_a_changed_tree_is_analysed_anew() {
	let mut context = Context::new();
	warp::analyzer::extract_user_functions(&mut context, &parse("f(x) := x; f(1)"));
	let mut changed = Context::new();
	warp::analyzer::extract_user_functions(&mut changed, &parse("f(x) := x; g(y) := y; f(1)"));
	assert!(!context.user_functions.contains_key("g"));
	assert!(changed.user_functions.contains_key("g"));
}

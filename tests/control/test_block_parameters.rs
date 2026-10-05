//! wiki/charged.md sections 4 and 5, stage 3: `code e` / `block e` are blocks, `data e` runs only with a warning, a `block`
//! parameter receives its argument unevaluated (lisp macros without macros), and any `!` on an expression is marked:
//! one known only at run time is a loud error until the run-time compiler exists
use crate::common::fails_with;
use warp::diagnostic::take_warnings;
use warp::is;
use warp::wasm_emitter::eval;

#[test]
fn test_code_and_block_prefixes_make_blocks() {
	is!("x = code 1+2; x!", 3);
	is!("a = 2; x = block a*10; a = 3; x!", 30);
}

#[test]
fn test_data_runs_with_a_warning() {
	take_warnings();
	is!("y = data 1+2; y!", 3);
	assert!(take_warnings().iter().any(|warning| warning.message.contains("running data as code")));
}

#[test]
fn test_block_parameters_take_their_argument_unevaluated() {
	is!("when_not(c, body:block) := if not c { body! } else { 0 }; when_not(0, 7)", 7);
	is!("when_not(c, body:block) := if not c { body! } else { 0 }; when_not(1, 7)", 0);
	is!("twice(body:block) := { body!; body! }; n = 0; twice(n += 5); n", 10);
	is!("x = 4; when_not(c, body:block) := if not c { body! } else { 0 }; when_not(0, x * 2)", 8);
}

#[test]
fn test_a_bang_known_only_at_run_time_is_loud() {
	is!("xs = [data a+1, data a*2]; a=5; xs#2!", 10); // runs at run time since run-block-5 (P73, notes/runtime_eval.md)
	is!("x=3; x!+1", 4);
	is!("x=\"hi\"; x.upper!; x", "HI");
}

#[test]
fn test_a_prefix_takes_the_rest_of_its_group() {
	is!("a = 1; count [data a]", 1);
	is!("q = 1; xs = [(data q+1), 2]; count xs", 2);
	assert_eq!(eval("[data 1/0]").serialize(), "[1/0]");
	assert_eq!(eval("x = (data a+1); x").serialize(), "a+1");
}

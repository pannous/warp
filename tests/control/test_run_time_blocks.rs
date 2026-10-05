// Run-time `!` (wiki/charged.md §5, notes/runtime_eval.md, package 4). Step 0, user 2026-10-05: "give a compiler
// warning if the code contains unresolved symbol arithmetic": a block over names the program defines nowhere.
use warp::diagnostic::take_warnings;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	warp::wasm_emitter::eval(code);
	take_warnings().into_iter().map(|warning| warning.message).collect()
}

#[test]
fn a_block_over_names_defined_nowhere_warns() {
	let warnings = warnings_of("x : a+b; x");
	assert!(warnings.iter().any(|message| message.contains("x keeps a+b over a, b, defined nowhere in the program")), "{warnings:?}");
	assert!(warnings_of("x : a*2 > limit; x").iter().any(|message| message.contains("over a, limit")));
}

#[test]
fn a_block_over_defined_names_does_not_warn() {
	assert!(warnings_of("a=1; x : a+b; b=2; x!").iter().all(|message| !message.contains("defined nowhere")));
	assert!(warnings_of("x : a+1; a=5; x!").iter().all(|message| !message.contains("defined nowhere")));
}

// Step 2: a block known only at run time runs through the host (run_block), natively
#[test]
fn interpret_runs_a_block_known_only_at_run_time() {
	warp::is!("xs = [data a+1, data a*2]; a = 5; interpret(xs#2)", 10);
	warp::is!("y = data 6*7; interpret y", 42);
	warp::is!("n=3; t=\"ab\"; ys = [data n+1, data t+\"c\"]; interpret(ys#2)", "abc");
}

#[test]
fn interpret_of_a_constant_block_runs_it_where_written() {
	warp::is!("a=1; x : a+b; b=2; interpret x", 3);
}

#[test]
fn a_run_time_block_reads_the_values_where_it_runs() {
	warp::is!("xs = [data a, data 0]; a = 1; r = interpret(xs#1); a = 2; r", 1);
}

#[test]
fn a_failing_run_time_block_names_the_block() {
	crate::common::fails_with("y = data q+1; interpret y", "the block q+1 failed: undefined variable: q");
}

// Step 3 and 5: running a block at run time is the effect Eval, and the block itself is pure by default
#[test]
fn running_a_run_time_block_is_the_eval_effect() {
	warp::is!("f(xs) := interpret(xs#1); effects of f", warp::Node::Symbol("Eval".into()));
}

#[test]
fn a_run_time_block_gets_no_capabilities() {
	crate::common::fails_with("y = data print(\"hi\"); interpret y", "a block run at run time is pure: print needs the wasi capability");
}

// Step 4: a compiled module that runs blocks at run time says it needs a warp host
#[test]
fn compiling_a_run_time_block_warns_about_the_host() {
	take_warnings();
	assert!(warp::pipeline::compile("xs = [data a+1, data a*2]; a = 5; interpret(xs#2)").is_ok());
	assert!(take_warnings().iter().any(|warning| warning.message.contains("its host must provide host.run_block")));
}

// The program's functions inside a run-time block, and exact numbers beyond the fixnums as its value
#[test]
fn a_run_time_block_calls_the_programs_functions() {
	warp::is!("def twice(x): x*2; y = data twice(21); interpret y", 42);
	warp::is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); k=10; y = data fib(k)+1; interpret y", 56);
	crate::common::fails_with("y = data nope(2); interpret y", "the block (nope 2) failed: undefined function: nope");
}

#[test]
fn a_run_time_block_hands_back_ratios_and_big_ints() {
	warp::is!("y = data 7/2; z = interpret y; z*2", 7);
	warp::is!("y = data 7/2; z = interpret y; z == 7/2", 1);
	warp::is!("y = data 2^70; z = interpret y; z+1 == 2^70+1", 1);
	warp::is!("y = data 0-2^70; z = interpret y; z == 0-2^70", 1);
}

// P73: `x!` forces; a block known only at run time runs through the host, an optional unwraps, a value is itself
#[test]
fn a_bang_known_only_at_run_time_runs_the_block() {
	warp::is!("xs = [data a+1, data a*2]; a = 5; xs#2!", 10);
	warp::is!("xs = [data a+1, data a*2]; a = 5; y = xs#2; y!", 10);
	warp::is!("5!", 5);
	crate::common::fails_with("xs=[1, ø]; xs#2!", "unwrapped ø");
	warp::is!("x = 3; x!", 3);
	crate::common::fails_with("x = ø; x!", "unwrapped ø");
}

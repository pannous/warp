// A function changes a main-level variable only when it is declared `global` (wiki/effects.md: State effect);
// without the declaration the compiler educates instead of shadowing or mutating silently. probes/globals/
use crate::is;
use warp::diagnostic::{with_warning_mode, WarningMode};
use crate::common;

#[test]
fn test_function_changes_a_declared_global() {
	is!("global n=0; def f(x){n+=1;x}; f(3); n", 1);
	is!("global n=0; fn f(x){n=x*2;x}; f(3); n", 6);
	is!("global n=0; f(x) := { n++; x }; f(3); f(4); n", 2);
	is!("global n=0; def f(x){n+=x;n}; f(3)+f(4)", 10);
	is!("global limit=10; def ok(x){x<limit}; ok(3)", 1);
}

#[test]
fn test_function_changes_global_floats_lists_and_texts() {
	is!("global t=0.5; def f(x){t+=x;x}; f(2.0); t", 2.5);
	is!("global xs=[1,2,3]; def f(i){xs[i]=9;i}; f(1); xs[1]", 9);
	is!("global counts=[0,0,0]; def bump(i){counts[i]+=1; i}; for k in [0,2,2] { bump(k) }; counts#3", 2);
	is!("global s=\"a\"; def f(x){s=s+\"b\";x}; f(1); s", "ab");
	is!("global xs=[1,2,3]; xs[1]*2", 4);
}

#[test]
fn test_changing_an_undeclared_main_variable_educates() {
	let hint = "n is a main-level variable: declare it `global n` to change it from a function, or use a new local name";
	common::fails_with("n=0; def f(x){n+=1;x}; f(3); n", hint);
	common::fails_with("n=0; def f(x){y=n; n=y+1; x}; f(3); n", "fix: global n");
	common::fails_with("n=0; f(x) := { n++; x }; f(3); n", hint);
	common::fails_with("xs=[1,2,3]; def f(i){xs[i]=9;i}; f(1); xs", "declare it `global xs`");
}

#[test]
fn test_own_names_and_reads_stay_fine() {
	is!("def f(){ m = 5; m }; f()", 5);
	is!("k=3; def f(x){x+k}; f(1)", 4);
	is!("x=1; def f(x){x=x+1; x}; f(5)", 6);
	is!("n=0; def f(x){let n=5;n}; f(3)", 5);
}

#[test]
fn test_fresh_binding_of_a_main_name_asks_local_or_global() {
	// unanswered: a warning taking the default, a new local of the function; main's name is unchanged
	is!("n=0; def f(x){n=5;x}; f(3); n", 0);
	is!("def f(){ total = 5; total }; total = f(); total", 5);
	// the explicit forms say each reading without a warning
	is!("global n=0; def f(x){n=5;x}; f(3); n", 5);
	is!("global xs=[1,2]; def f(){xs=[7,8,9];0}; f(); count(xs)", 3);
	is!("n=0; def f(x){let n=5;x}; f(3); n", 0);
	with_warning_mode(WarningMode::Error, || common::fails_with("n=0; def f(x){n=5;x}; f(3); n", "fix: let n = …"));
}

#[test]
fn test_print_statement_in_a_function() {
	is!("f(x) := { print(\"called\"); x }; f(3)", 3);
	is!("def f(x){ print(\"called\"); return x }; f(3)", 3);
}

#[test]
fn test_reassigning_from_a_rounding_builtin() {
	is!("x=10; x=floor(x/2)", 5);
	is!("x=10; x=floor(x/2)+1", 6);
	is!("x=10; while x>1 { x=floor(x/2) }; x", 1);
	is!("x=10; x=ceil(x/3)", 4);
}

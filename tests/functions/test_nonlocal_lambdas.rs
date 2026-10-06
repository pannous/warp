// A lambda that changes a variable of its enclosing function shares it, as in JS, Kotlin, Swift, C#, Ruby and Julia
// counters (P124 asked; an existing test pins the returned counter working); `nonlocal n` may be written. A nested
// def keeps Python's rule: it declares `nonlocal n`, else the change is an error naming the fix, never a silent
// restart from the captured value
use crate::common::fails_with;
use crate::is;

#[test]
fn a_lambda_changing_an_enclosing_variable_shares_it() {
	is!("def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c()", 2);
	is!("def counter(){ n=0; return () => { n = n + 1; return n } }; c=counter(); c(); c(); c()", 3);
	is!("def acc(){ total=0; x=>{ total+=x; total } }; a=acc(); a(5); a(10)", 15);
	is!("def counter(){ n=0; ()=>{ n+=1; n } }; a=counter(); b=counter(); a(); a(); b()", 1);
}

#[test]
fn a_lambda_may_declare_nonlocal() {
	is!("def counter(){ n=0; ()=>{ nonlocal n; n+=1; n } }; c=counter(); c(); c()", 2);
}

#[test]
fn a_nested_def_changing_it_without_nonlocal_is_an_error() {
	fails_with("def counter(){ n=0; def inc(){ n+=1; n }; function inc }; c=counter(); c(); c()", "nonlocal n");
	is!("def make(){ n=0; def inc(){ nonlocal n; n += 1; n }; return function inc }; c=make(); c(); c()", 2);
}

#[test]
fn a_fresh_local_of_the_same_name_is_no_change() {
	is!("def f(){ n=1; g = () => { n = 5; n }; g() + n }; f()", 6);
}

// a lambda's own fresh variable is no capture: assigning it changes nothing outside (no `global` asked)
#[test]
fn a_lambda_binds_its_own_locals() {
	is!("def apply(f, x) { f(x) }; apply(x => { y = x * 2; y + 1 }, 3)", 7);
	is!("def mk() { k => { t = k * 2; t } }; g = mk(); g(4)", 8);
}

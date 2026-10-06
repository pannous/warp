// Variadic (rest) parameters and spreading, as Julia `f(xs...)`, Swift `f(_ xs: Int...)`, JS `f(...xs)`, Python
// `f(*xs)`, Kotlin `vararg xs`, C# `params int[] xs`: the rest parameter is a list of the arguments the others leave,
// and a list spreads into the parameters of a call (both resolved at compile time, a plain call)
use crate::is;
use warp::ints;

#[test]
fn a_rest_parameter_collects_the_remaining_arguments() {
	is!("def total(xs...){sum xs}; total(1,2,3)", 6); // Julia total(xs...) = sum(xs)
	is!("def total(...xs){sum xs}; total(1,2,3)", 6); // JS function total(...xs)
	is!("def total(*xs){sum xs}; total(1,2,3)", 6); // Python def total(*xs)
	is!("def count(xs...){#xs}; count(1,2,3,4)", 4);
	is!("def rest(a, more...){more}; rest(7,8,9)", ints(vec![8, 9]));
	is!("def first(a, rest...){a}; first(7,8,9)", 7);
}

#[test]
fn a_rest_parameter_may_be_empty() {
	is!("def count(a, rest...){#rest}; count(7)", 0);
	is!("def total(xs...){sum xs}; total()", 0);
}

#[test]
fn calls_without_parentheses_fill_the_rest_too() {
	is!("total(xs...) := sum xs; total 1 2 3", 6);
}

#[test]
fn a_list_spreads_into_a_rest_parameter() {
	is!("def total(xs...){sum xs}; ys=[1,2,3]; total(...ys)", 6); // JS total(...ys)
	is!("def total(xs...){sum xs}; ys=[1,2,3]; total(ys...)", 6); // Julia total(ys...)
	is!("def total(xs...){sum xs}; ys=[1,2,3]; total(*ys)", 6); // Python total(*ys)
	is!("def total(xs...){sum xs}; ys=[2,3]; total(1, ...ys)", 6);
}

#[test]
fn a_list_spreads_into_fixed_parameters() {
	is!("def f(a,b,c){a*100+b*10+c}; xs=[1,2,3]; f(...xs)", 123);
	is!("def f(a,b,c){a*100+b*10+c}; xs=[2,3]; f(1, ...xs)", 123);
}

#[test]
fn kotlin_vararg_and_csharp_params() {
	is!("fun total(vararg xs: Int): Int = xs.sum(); total(1, 2, 3)", 6);
	is!("int Total(params int[] xs) { return xs.sum(); }; Total(1, 2, 3)", 6);
}

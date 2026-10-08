// Named arguments (user, P37): f(name=value) and f(name:value) set a parameter or a free variable of the body; a
// missing argument stays an error, a variable of the same name is never captured for a parameter
use crate::is;

#[test]
fn named_arguments_set_parameters_in_any_order() {
	is!("f(a, b) := a - b; f(b=1, a=5)", 4);
	is!("f(a, b) := a - b; f(b:1, a:5)", 4);
	is!("f(a, b) := a - b; f(5, b=1)", 4);
}

#[test]
fn named_arguments_follow_the_parameter_order_of_a_python_colon_def() {
	is!("def g(a, b, c): return a*100+b*10+c; g(1, c=9, b=2)", 129);
	is!("def g(a, b, c):\n    return a*100+b*10+c\ng(1, c=9, b=2)", 129);
	is!("def g(a, b=5, c=7): return a*100+b*10+c; g(1, c=9)", 159);
}

#[test]
fn named_arguments_set_free_variables_of_the_body() {
	is!("f y := y*y+v; f(y=2, v=3)", 7);
	is!("g={x*y}; g(x:2 y:3)", 6);
	is!("v=10; f(y) := y+v; f(y=1) + f(y=1, v=2)", 14);
}

#[test]
fn a_missing_or_unknown_argument_is_an_error() {
	crate::common::fails_with("x=7; f(x):=x*x; f()", "needs a value for parameter x");
	crate::common::fails_with("f(a, b) := a - b; f(b=1)", "needs a value for parameter a");
	crate::common::fails_with("f(a) := a; f(a=1, c=2)", "f has no parameter c");
}

#[test]
fn named_arguments_run_as_written() {
	// P216: left to right as written, not in parameter order: h runs first (i=2), then g (i=21)
	let counters = "global i = 0; g() := { i = i * 10 + 1; i }; h() := { i = i * 10 + 2; i }; f(a, b) := a * 100 + b; ";
	is!(&format!("{counters}f(b=h(), a=g())"), 2102);
	is!(&format!("{counters}f(a=g(), b=h())"), 112);
	is!(&format!("{counters}k() := f(b=h(), a=g()); k()"), 2102);
}


#[test]
fn a_bare_flag_name_in_a_call_sets_it() {
	// card copy-shallow: `copy(shallow)` is `copy(shallow: true)`, and so for any flag (a parameter of default yes/no)
	is!("a = {x: {y: 1}}; b = a.copy(shallow); b.x.y = 5; a.x.y", 5);
	is!("f(a, loud = false) := if loud then a * 10 else a; f(3, loud)", 30);
	is!("f(a, loud = false) := if loud then a * 10 else a; f(3)", 3);
	is!("f(a, loud: false) := if loud then a * 10 else a; f(3, loud)", 30);
	// a variable of that name is passed as it is
	is!("f(a, loud = false) := if loud then a * 10 else a; loud = false; f(3, loud)", 3);
	is!("f(a, loud = false) := if loud then a * 10 else a; g(loud) := f(2, loud); g(false)", 2);
}

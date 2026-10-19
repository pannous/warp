// A function passed by name to a higher-order function is called by name in a copy made for it (card
// specialise-named-functions; probes/perf/named_function_argument.warp measures it): the program calls the copy
use crate::is;
use warp::parse;

const EACH: &str =
	"ink(x, y) := (x * 3 + y) % 7; each(f, n) := { total = 0; for i in 0..n { total = total + f(i, i + 1) }; total }; each(ink, 10)";
const CASES: &str = "dbl(x) := x * 2; inc(x) := x + 1
times(f, n, x) := if n < 1 then x else times(f, n - 1, f(x))
twice(f, x) := f(f(x))
via(f, x) := twice(f, x)
both(f, g, x) := g(f(x))
shadow(ink, x) := ink(x)
ink(x) := x * 10
times(dbl, 3, 1) * 1000 + via(inc, 5) * 100 + both(dbl, inc, 4) * 10 + shadow(inc, 7)";

#[test]
fn a_function_passed_by_name_is_called_by_name() {
	is!(EACH, 29);
}

#[test]
fn recursive_passed_on_and_several_functions_by_name() {
	is!(CASES, 8798);
	// a lambda is still a closure
	is!("twice(f, x) := f(f(x)); twice(x => x * 3, 2)", 18);
	// a name the program also assigns is a variable there, not the function
	is!("inc(x) := x + 1; twice(f, x) := f(f(x)); g = &inc; twice(g, 1)", 3);
}

#[test]
fn a_function_parameter_passed_on_in_a_method_call() {
	// card forward-function: the copy renames the parameter in method arguments too (`xs.map(f)` keeps `.map`)
	is!("inc(x) := x + 1; a(xs, f) := xs.map(f); a([1, 2], inc)", parse("[2 3]"));
	is!("inc(x) := x + 1; a(xs, f) := xs.map(x => f(x)); a([1, 2], inc)", parse("[2 3]"));
	// the shared tree walk of a self-hosted pass: f handed down through a lambda
	const WALK: &str = "walk(n, f) := { if n is list { return f(n.map(x => walk(x, f))) }; f(n) }
bump(n) := if n is list then n else n + 1
walk([1, [2, 3]], bump)";
	is!(WALK, parse("[2 [3 4]]"));
}

//! List literals with computed elements, min/max of any expressions, and `+=` next to a function with a same-named local
//! (Levenshtein field test, probes/listexpr/)

use warp::is;

#[test]
fn a_list_literal_keeps_its_computed_elements() {
	is!("p=[5,6,7]; c=[p[1]]; c[0]", 6);
	is!("p=[5,6,7]; d=[1] + [p[1]]; count(d)", 2);
	is!("s=\"abc\"; c=[#s]; c#1", 3);
	is!("c=[1, if 2>1 then 5 else 6]; c#2", 5);
}

#[test]
fn push_takes_any_expression() {
	is!("p=[5,6,7]; c=[1]; c.push(p[1]); c#2", 6);
	is!("p=[5,6,7]; c=[]; c.push(p[1] + 1); c#1", 7);
	is!("c=[1]; x=2; y=3; c.push(min(x, y)); c#2", 2);
}

#[test]
fn min_and_max_take_any_expressions() {
	is!("p=[1,2]; min(p#1, 5)", 1);
	is!("p=[4,2,3]; min(p#1, p#2, p#3)", 2);
	is!("def f(x){x*2}; min(f(3), 5)", 5);
	is!("def g(p){ min(p#1, p#2) }; g([7,3])", 3);
	is!("p=[1,8]; max(min(p#1, 5), p#2 - 1)", 7);
	is!("min(4, 2, 3)", 2);
	is!("max([1, 5, 2])", 5);
	is!("xs=[3,1,2]; min(xs)", 1);
	is!("def f(x){[x, 9, 4]}; min(f(3))", 3);
}

#[test]
fn a_function_local_does_not_leak_into_the_caller() {
	is!("def g(a) { t = a; 7 }; t = 0; t += g(\"x\"); t", 7);
	is!("def g(a) { t = a; 7 }; t = 0; t += 7; t", 7);
	is!("def g(a) { t = a; 7 }; t = 0; t = t + g(\"x\"); t", 7);
}

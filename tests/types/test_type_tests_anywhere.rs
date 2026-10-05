//! `x is T` anywhere: in conditions, and-chains, ternaries and function bodies, answered from the static type when it is
//! known and from the value's kind at run time when it is not (an item of a mixed list, a Node)
use warp::is;

#[test]
fn test_type_tests_in_conditions_and_chains() {
	is!("x=[1]; if x is list and count(x)==1 {1} else {2}", 1);
	is!("x=[1]; (x is list) and 1", 1);
	is!("x=3; if x is int then 10 else 20", 10);
	is!("x=3; x is text ? 1 : 2", 2);
	is!("s=0; i=0; while i < 3 and i is int { i += 1; s += i }; s", 6);
}

#[test]
fn test_items_of_a_mixed_list_are_tested_at_run_time() {
	is!("xs=[1, \"a\"]; xs#2 is text", 1);
	is!("xs=[1, \"a\"]; xs#1 is text", 0);
	is!("xs=[1, \"a\", 2.5]; n=0; for x in xs { if x is number { n += 1 } }; n", 2);
	is!("xs=[1, [2]]; xs#2 is list", 1);
	is!("x = a:1; x is pair", 1);
}

#[test]
fn test_type_tests_in_function_bodies() {
	is!("f(x) := if x is text then 1 else 0; f(\"a\") + f(\"b\")", 2);
}

//! card arctan-allow (user): the definition `arctan := arc_tangent` names the function again, as `arctan = &arc_tangent`
//! does; the assignment `g = add` stays P83's error with the fix. `inc := add 1` leaving an argument out is an error whose
//! fix is the partial application with its hole, `inc := add(1, _)`
use crate::common::fails_with;
use crate::is;

const ARC_TANGENT: &str = "arc_tangent(x) := atan(x); arctan := arc_tangent; ";

#[test]
fn a_defined_name_of_a_function_calls_it() {
	is!(&format!("{ARC_TANGENT}arctan(1) == arc_tangent(1)"), true);
	is!("arc_sine(x) := asin(x); arcsin := arc_sine; arcsin(1) == asin(1)", true);
	is!("arc_cosine(x) := acos(x); arccos := arc_cosine; arccos(1)", 0.0);
	is!("doubled(x) := x*2; twice := doubled; twice(3) + twice 4", 14);
	is!("doubled(x) := x*2; twice := doubled; [1 2].map(twice)", warp::parse("[2 4]"));
}

#[test]
fn an_assigned_bare_name_stays_the_error() {
	fails_with("doubled(x) := x*2; twice = doubled; twice(3)", "fix: function doubled");
}

// partial application needs the explicit hole (user 2026-10-10, option C): `inc := add(1, _)`
#[test]
fn a_partial_definition_names_its_hole() {
	is!("add(a, b) := a + b; inc := add(1, _); inc 5", 6);
	is!("add(a, b, c) := a + b + c; f := add(1, _, _); f(2, 3)", 6);
	fails_with("add(a, b) := a + b; inc := add 1; inc 5", "fix: inc := add(1, _)");
	is!("add(a, b = 2) := a + b; three := add 1; three", 3);
}

// card alias-chain: an alias of an alias, called braceless, calls the function
#[test]
fn an_alias_of_an_alias_calls_the_function() {
	is!("doubled(x) := x*2; g := &doubled; h := g; h 4", 8);
	is!("doubled(x) := x*2; g := doubled; h := g; h(4) + g 1", 10);
}

//! Python-style unpacking assignments in wasp: tuples, lists, starred rest, nesting, errors.
//! Each case is the Python program's result; `#[ignore = "next"]` marks what does not work yet.

use crate::common::fails_with;
use warp::is;

#[test]
fn probe_unpack_tuple_values() {
	is!("a, b = 1, 2; a*10+b", 12); // Python: a, b = 1, 2
}

#[test]
#[ignore = "next"]
fn probe_unpack_parenthesized_tuple() {
	is!("a, b = (1, 2); a*10+b", 12); // Python: a, b = (1, 2)
}

#[test]
#[ignore = "next"]
fn probe_unpack_list_literal() {
	is!("a, b = [1, 2]; a*10+b", 12); // Python: a, b = [1, 2]
}

#[test]
#[ignore = "next"]
fn probe_unpack_list_variable() {
	is!("xs = [1, 2]; a, b = xs; a*10+b", 12); // Python: xs = [1, 2]; a, b = xs
}

#[test]
fn probe_unpack_tuple_function() {
	is!("f() := return 1, 2; a, b = f(); a*10+b", 12); // Python: def f(): return 1, 2
}

#[test]
#[ignore = "next"]
fn probe_unpack_list_function() {
	is!("f() := [1, 2]; a, b = f(); a*10+b", 12); // Python: def f(): return [1, 2]
}

#[test]
fn probe_swap() {
	is!("a=1; b=2; a, b = b, a; a*10+b", 21); // Python: a, b = b, a
}

#[test]
#[ignore = "next"]
fn probe_list_target() {
	is!("[a, b] = [1, 2]; a*10+b", 12); // Python: [a, b] = [1, 2]
}

#[test]
#[ignore = "next"]
fn probe_parenthesized_target() {
	is!("(a, b) = 1, 2; a*10+b", 12); // Python: (a, b) = 1, 2
}

#[test]
#[ignore = "next"]
fn probe_star_rest_from_list() {
	is!("a, *rest = [1, 2, 3]; rest == [2, 3]", 1); // Python: rest == [2, 3]
	is!("a, *rest = [1, 2, 3]; a", 1);
}

#[test]
#[ignore = "next"]
fn probe_star_rest_from_values() {
	is!("a, *rest = 1, 2, 3; rest == [2, 3]", 1); // Python: a, *rest = 1, 2, 3
}

#[test]
#[ignore = "next"]
fn probe_star_init() {
	is!("*init, last = [1, 2, 3]; init == [1, 2] and last == 3", 1); // Python: *init, last = [1, 2, 3]
}

#[test]
#[ignore = "next"]
fn probe_star_middle() {
	is!("a, *mid, z = [1, 2, 3, 4]; mid == [2, 3] and a+z == 5", 1); // Python: a, *mid, z = [1, 2, 3, 4]
}

#[test]
#[ignore = "next"]
fn probe_star_rest_empty() {
	is!("a, *rest = [1]; rest == []", 1); // Python: rest == []
}

#[test]
#[ignore = "next"]
fn probe_star_rest_from_tuple_function() {
	is!("f() := return 1, 2, 3; a, *rest = f(); rest == [2, 3]", 1); // Python: a, *rest = f()
}

#[test]
#[ignore = "next"]
fn probe_nested_unpacking() {
	is!("(a, b), c = (1, 2), 3; a*100+b*10+c", 123); // Python: (a, b), c = (1, 2), 3
}

#[test]
#[ignore = "next"]
fn probe_unpack_text() {
	is!("a, b = \"hi\"; a", "h"); // Python: a, b = "hi" → 'h', 'i'
}

#[test]
fn probe_underscore_discards() {
	is!("f() := return 1, 2; _, b = f(); b", 2); // Python: _, b = f()
}

#[test]
fn probe_too_many_values_is_loud() {
	fails_with("a, b = [1, 2, 3]", "values"); // Python: ValueError: too many values to unpack (expected 2)
	fails_with("a, b = 1, 2, 3", "values");
}

#[test]
fn probe_too_few_values_is_loud() {
	fails_with("a, b, c = [1, 2]", "values"); // Python: ValueError: not enough values to unpack (expected 3, got 2)
	fails_with("xs = [1, 2]; a, b, c = xs", "values");
}

#[test]
fn probe_for_loop_unpacking() {
	is!("s = 0; for a, b in [[1, 2], [3, 4]] { s += a*b }; s", 14); // Python: for a, b in [[1, 2], [3, 4]]
}

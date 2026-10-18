//! Card typed-builtins: a built-in word of several `from warp` lines in lib/prelude.warp is overloaded by its first
//! parameter's type, and the line of the argument's type gives the call's result, as a program's own overloads do
use crate::is;

#[test]
fn the_overload_of_the_argument_type_gives_the_result() {
	is!("reverse(\"abc\") + \"d\"", "cbad");
	is!("x = reverse([1 2]); x#1", 2);
	is!("sort(\"cba\") + \"!\"", "abc!");
	is!("sorted(\"cba\") + \"!\"", "abc!");
	is!("\"hello\".slice(1, 3) + \"!\"", "el!");
	is!("xs = [1 2 3].slice(1); xs#1", 2);
}

#[test]
fn a_value_known_at_run_time_takes_its_overload_then() {
	is!("f(x: any) := reverse(x); f(\"ab\")", "ba");
	is!("f(x: any) := reverse(x); f([1 2]) == [2 1]", true);
}

#[test]
fn each_overload_type_is_the_librarys() {
	crate::common::fails_with("reverse(t: text) := 1; reverse(\"ab\")", "reverse of a text is the library's word");
	crate::common::fails_with("sort(xs: list) := 1; sort([2 1])", "sort of a list is the library's word");
}

#[test]
fn a_value_no_overload_is_typed_for_takes_the_first_line() {
	let program = warp::pipeline::lower("a=[3,1,2,7]; b=a[0:2]; len(b)").expect("lowered");
	let kinds = warp::analyzer::inferred_kinds(&program);
	assert!(kinds.contains(&"b: list".to_string()), "{kinds:?}");
}

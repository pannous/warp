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

#[test]
fn the_member_words_declare_their_signatures_in_the_prelude() {
	is!("filter.signature", warp::Node::Text("(xs, f:function)".into()));
	is!("insert.params", warp::texts(vec!["xs", "x", "at"]));
	is!("any.signature", warp::Node::Text("(xs, f:function) -> bool".into()));
}

#[test]
fn a_member_words_number_or_bool_result_is_decided_where_it_is_made() {
	is!("x = number(\"2.5\"); x + 1", 3.5);
	is!("any([1 2], x => x > 1)", true);
}

#[test]
fn a_result_read_off_an_argument_is_spelled_same_as_or_value_of() {
	is!("field_with.signature", warp::Node::Text("(object, name, value) -> same as object".into()));
	is!("map_get_or.signature", warp::Node::Text("(m, key, default = ø) -> value of m".into()));
	is!("list_become.signature", warp::Node::Text("(place, fresh) -> same as place".into()));
	is!("removed_value.signature", warp::Node::Text("(x, k) -> any".into()));
	is!("list_sorted_by.signature", warp::Node::Text("(xs, keys) -> any".into()));
	let program =
		warp::pipeline::lower("p = {x: 1}; q = p.copy(); m = {a: \"x\"}; f = m.get(\"a\", \"z\")").expect("lowered");
	let kinds = warp::analyzer::inferred_kinds(&program);
	assert!(kinds.contains(&"f: text".to_string()) && !kinds.contains(&"q: int".to_string()), "{kinds:?}");
}

#[test]
fn count_declares_a_list_a_text_and_a_map_overload() {
	is!("count.signature", warp::Node::Text("(xs:list) -> int".into()));
	is!("count(\"ab\") + count({a: 1})", 3);
}

#[test]
fn a_typed_builtin_called_without_parentheses_is_the_call() {
	is!("sum.signature", warp::Node::Text("(xs:list) -> element of xs".into()));
	is!("xs = [1 2 3]; sum xs", 6);
	is!("xs = [1.5 2]; sum xs", 3.5);
}

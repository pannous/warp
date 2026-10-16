//! Data stays clean (decision error-optional): an Error written into a list item raises, with its own message; results,
//! locals and arguments carry it
use crate::common::fails_with;
use crate::is;

const E: &str = "e = try 10/0; ";

#[test]
fn test_an_error_in_a_list_literal_raises() {
	fails_with(&format!("{E}xs = [e 2]; xs"), "divide by zero: a list item holds no error");
	fails_with(&format!("{E}f(x) := x; xs = [1 f(e)]; xs"), "a list item holds no error");
	fails_with(&format!("{E}xs = [if 1 then e else 1, 2]; xs"), "a list item holds no error");
	fails_with(&format!("{E}c = 1; xs = [c ? e : 1, 2]; xs"), "a list item holds no error");
}

#[test]
fn test_a_list_of_values_and_a_handled_error_stay() {
	is!("f(x) := x; xs = [f(1) 2]; xs#1", 1);
	is!(&format!("{E}xs = [(try e else 0) 2]; xs#1"), 0);
}

#[test]
fn test_try_catches_an_error_in_data() {
	is!(&format!("{E}r = try [e 2]; if r failed then 1 else 2"), 1);
}

#[test]
fn test_an_error_still_travels_as_an_argument_and_a_local() {
	is!(&format!("{E}f(x) := if x failed then 1 else 2; f(e)"), 1);
	is!(&format!("{E}g = e; if g failed then 1 else 2"), 1);
}

#[test]
fn test_an_error_written_into_a_list_or_field_raises() {
	fails_with(&format!("{E}xs = [1, 2]; xs#1 = e; xs"), "divide by zero: a list item holds no error");
	fails_with(&format!("{E}xs = [1, 2]; xs[0] = e; xs"), "a list item holds no error");
	fails_with(&format!("{E}xs = [1, 2]; xs.add(e); xs"), "a list item holds no error");
	fails_with(&format!("{E}q = {{k: 1}}; q.k = e; q"), "divide by zero: a field holds no error");
	fails_with(&format!("{E}q = {{k: e}}; q"), "a field holds no error");
	fails_with(&format!("{E}class pt{{x:int}}; p = pt(e); p"), "a field holds no error");
}

#[test]
fn test_a_place_declared_or_error_holds_an_error() {
	is!(&format!("{E}xs:[int or error] = [e, 2]; if xs#1 failed then 1 else 2"), 1);
	is!(&format!("{E}xs:[int?] = [e, 2]; if xs#1 failed then 1 else 2"), 1);
	is!(&format!("{E}xs:[int or error] = [1, 2]; xs#1 = e; if xs#1 failed then 1 else 2"), 1);
	is!(&format!("{E}class pt{{x:int or error}}; p = pt(e); if p.x failed then 1 else 2"), 1);
}

#[test]
fn test_an_error_written_into_a_tuple_entry_or_inserted_raises() {
	fails_with(&format!("{E}t = (e, 2); t"), "divide by zero: a list item holds no error");
	fails_with(&format!("{E}xs = [1]; xs.insert(1, e); xs"), "a list item holds no error");
	fails_with(&format!("{E}xs = [1]; xs.insert(e, at: 1); xs"), "a list item holds no error");
	fails_with(&format!("{E}m = {{a: 1}}; m[\"k\"] = e; m"), "a field holds no error");
	is!(&format!("{E}a, b = e, 2; if a failed then 1 else 2"), 1);
}

#[test]
fn test_only_the_written_place_declared_or_error_holds_an_error() {
	fails_with(&format!("{E}f(x: int?) := x; q = {{x: 1}}; q.x = e; q"), "a field holds no error");
	fails_with(&format!("{E}class P{{x: int?}}; q = {{x: 1}}; q.x = e; q"), "a field holds no error");
	is!(&format!("{E}class P{{x: int?}}; p = P(1); p.x = e; if p.x failed then 1 else 2"), 1);
	is!(&format!("{E}f(xs: [int or error], v) := {{xs#1 = v; xs}}; count(f([1, 2], e))"), 2);
	fails_with(&format!("{E}f(xs: [int], v) := {{xs#1 = v; 3}}; f([1, 2], e)"), "a list item holds no error");
}

#[test]
fn test_an_error_written_by_a_variable_key_into_a_map_names_the_field() {
	fails_with(&format!("{E}m = {{a: 1}}; k = \"b\"; m[k] = e; m"), "divide by zero: a field holds no error");
}

#[test] // cards writing-error-list, parameter-declared: a list-of-union parameter takes the list, not each item
fn test_a_parameter_declared_a_list_of_a_union_takes_index_writes() {
	is!("f(xs: [int or text], v) := {xs#1 = v; xs}; f([1, 2], 'a')#1", "a");
	is!(&format!("{E}g(ys: [int or error]) := {{ys#1 = e; ys}}; if g([1])#1 failed then 1 else 2"), 1);
	is!("f(xs: [int or text], v) := {xs#1 = v; 3}; f([1, 2], 7)", 3);
}

//! Newcomer syntax from Python/JS is eaten: it compiles to its intent, and a hint names the wasp form
//! (probes/sugar/cases.txt; decisions in notes/open_decisions.md)

mod common;
use common::fails_with;
use warp::*;

#[test]
fn let_var_and_const_declare_inside_any_block() {
	is!("fun f(n) { let m = n + 1; return m }; f(2)", 3);
	is!("fun f(n) { var m = n + 1; return m }; f(2)", 3);
	is!("fun f(n) { const m = n + 1; return m }; f(2)", 3);
	is!("s=0; for i in 1…3 { let t = i; s += t }; s", 6);
	is!("if 1 { let y = 2; y } else { 0 }", 2);
	is!("let x = 3; x", 3);
	is!("let x = 0; x += 1; x", 1); // JS intent: let is reassignable
}

#[test]
fn an_unknown_word_as_statement_is_loud() {
	fails_with("foo x = 3; x", "undefined variable: foo");
	fails_with("x=1; foo; x", "undefined variable: foo");
	fails_with("x=1\nfoo\nx", "undefined variable: foo");
	is!("a;b;c", "c"); // words alone are data
}

#[test]
fn len_counts() {
	is!("len([1,2,3])", 3);
	is!("len(\"hello\")", 5);
	is!("xs=[1 2 3]; len(xs)", 3);
	is!("len([])", 0);
}

#[test]
fn a_one_element_list_times_n_repeats_it() {
	is!("[0]*3", ints(vec![0, 0, 0]));
	is!("3*[1]", ints(vec![1, 1, 1]));
	is!("n=4; xs=[0]*n; xs.length", 4);
	is!("def f(n){ xs=[0]*n; xs#2=5; xs }; f(3)", ints(vec![0, 5, 0]));
	is!("3 times [0]", ints(vec![0, 0, 0]));
	is!("x = 100 times [0]; x.length", 100);
	fails_with("[1 2]*2", "type error"); // a longer list is no repetition
}

#[test]
fn the_empty_list_counts_zero_and_grows() {
	is!("[].length", 0);
	is!("b=[]; b.count", 0);
	is!("b=[]; b.push(0); b.count", 1);
	is!("b=[]; b.push(0); b", ints(vec![0]));
}

#[test]
fn insert_takes_wasp_and_python_argument_orders() {
	is!("xs=[1 2]; xs.insert(0, 4); xs", ints(vec![4, 1, 2])); // Python: position, value
	is!("xs=[1 2]; xs.insert(4, 0); xs", ints(vec![4, 1, 2])); // wasp: value, position
	is!("pixel=[1 2 3]; pixel.insert(4,-1); pixel", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3]; pixel.insert(4); pixel", ints(vec![1, 2, 3, 4]));
	is!("xs=[1 2]; i=1; v=9; xs.insert(i, v); xs", ints(vec![1, 9, 2]));
	is!("b=[]; b.insert(0, 5); b", ints(vec![5]));
}

#[test]
fn a_number_joins_a_text_in_its_text_form() {
	is!("\"F:\" + 13", "F:13");
	is!("13 + \"F\"", "13F");
	is!("i=3; \"row \" + i + \":\"", "row 3:");
	is!("s=\"n=\"; s += 4; s", "n=4");
	fails_with("\"5\"*3", "type error");
}

#[test]
fn glued_double_slash_is_floor_division() {
	is!("7//2", 3);
	is!("-7//2", -4);
	is!("x=10; x//=2; x", 5);
	is!("7 // a comment", 7);
	is!("u='http://x.org'; 1", 1);
	is!("7 div 2", 3);
	is!("n=9; n div 2", 4);
}

#[test]
fn a_fractional_index_says_so() {
	fails_with("xs=[1 2 3]; n=5; xs[n/2]", "index must be an integer");
	fails_with("xs=[1 2 3]; n=5; xs#(n/2)", "index must be an integer");
	is!("xs=[1 2 3]; n=4; xs[n/2]", 3);
	is!("xs=[1 2 3 4]; n=5; xs[n//2]", 3);
}

#[test]
fn a_glued_hash_name_statement_counts() {
	is!("def g(a){ s=a\n #s }; g(\"abc\")", 3);
	is!("xs=[1 2 3]\n#xs", 3);
	is!("x=4 # a comment\nx", 4);
	is!("x=4\n#use lib\nx", 4); // a directive line stays a comment
}

//! Newcomer syntax from Python/JS is eaten: it compiles to its intent, and a hint names the wasp form
//! (probes/sugar/cases.txt; decisions in notes/open_decisions.md)

mod common;
use common::fails_with;
use warp::*;
use warp::node::strings;

#[test]
fn let_var_and_const_declare_inside_any_block() {
	is!("fun f(n) { let m = n + 1; return m }; f(2)", 3);
	is!("fun f(n) { var m = n + 1; return m }; f(2)", 3);
	is!("fun f(n) { const m = n + 1; return m }; f(2)", 3);
	is!("s=0; for i in 1…3 { let t = i; s += t }; s", 6);
	is!("if 1 { let y = 2; y } else { 0 }", 2);
	is!("let x = 3; x", 3);
	// wiki/variable.md: `let` binds once (unlike JS); `var` and plain `=` change
	is!("var x = 0; x += 1; x", 1);
	fails_with("let x = 0; x += 1; x", "x is let (immutable), cannot assign it again");
	fails_with("let x = 1; x = 2", "fix: declare it with var or plain `x =` if it changes");
	fails_with("let x = 1; x++", "x is let (immutable)");
	fails_with("let xs = [1 2]; xs#1 = 5", "xs is let (immutable)");
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
fn n_times_a_one_element_list_repeats_it() {
	is!("3 times [0]", ints(vec![0, 0, 0]));
	is!("x = 100 times [0]; x.length", 100);
	is!("def f(n){ xs = n times [0]; xs#2=5; xs }; f(3)", ints(vec![0, 5, 0]));
	// wiki/Footguns.md "Lists and arithmetic": Python repeats, NumPy multiplies, so `*` is refused with both readings
	fails_with("[0]*3", "ambiguous: Python repeats the list, NumPy multiplies each element; write `n times [x]`");
	fails_with("3*[1]", "write `n times [x]` to repeat");
	fails_with("[1 2]*2", "type error");
}

#[test]
fn the_empty_list_counts_zero_and_grows() {
	is!("[].length", 0);
	is!("b=[]; b.count", 0);
	is!("b=[]; b.push(0); b.count", 1);
	is!("b=[]; b.push(0); b", ints(vec![0]));
}

#[test]
fn insert_never_guesses_the_argument_order() {
	is!("xs=[1 2]; xs.insert(4, at: 0); xs", ints(vec![4, 1, 2]));
	is!("xs=[1 2]; i=1; xs.insert(9, at: i); xs", ints(vec![1, 9, 2]));
	is!("pixel=[1 2 3]; pixel.insert(4, at: -1); pixel", ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3]; pixel.insert(4); pixel", ints(vec![1, 2, 3, 4]));
	is!("b=[]; b.insert(5, at: 0); b", ints(vec![5]));
	// the kinds decide: the one Int is the position, in either order
	is!("xs=[\"a\" \"b\"]; xs.insert(0, \"z\"); xs", strings(vec!["z", "a", "b"]));
	is!("xs=[\"a\" \"b\"]; xs.insert(\"z\", 1); xs", strings(vec!["a", "z", "b"]));
	// two Ints: Python `insert(i, x)` and wasp `insert(x, i)` disagree (wiki/Footguns.md "Guessing intent")
	fails_with("xs=[1 2]; xs.insert(0, 4); xs", "ambiguous: insert(0, 4) inserts 4 at 0 in Python, 0 at 4 in wasp");
	fails_with("xs=[1 2]; i=1; v=9; xs.insert(i, v); xs", "write insert(v, at: i) or insert(i, at: v)");
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

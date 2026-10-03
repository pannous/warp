//! Newcomer syntax from Python/JS is eaten: it compiles to its intent, and a hint names the wasp form
//! (probes/sugar/cases.txt; decisions in notes/open_decisions.md)

use crate::common::fails_with;
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
	// wiki/Footguns.md "Lists and arithmetic": Python repeats, NumPy multiplies, so `*` is an Ask; unanswered an error
	fails_with("[0]*3", "type error: list * number: does `[0]*3` repeat the list or multiply each element? (too ambiguous to guess)");
	fails_with("3*[1]", "`3 times [1]` for repeat the list or `[1].map(x => x*3)` for multiply each element");
	fails_with("[1 2]*2", "too ambiguous to guess");
	fails_with("xs=[1 2]; xs*2", "type error"); // a list variable: no Ask yet
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
	// unanswered the Ask is an error naming both explicit forms
	fails_with("xs=[1 2]; xs.insert(0, 4); xs", "does insert(0, 4) put 4 at 0 (Python) or 0 at 4 (wasp)? (too ambiguous to guess)");
	fails_with("xs=[1 2]; i=1; v=9; xs.insert(i, v); xs", "`insert(v, at: i)` for position first, as Python or `insert(i, at: v)` for value first, as wasp");
}

#[test]
fn an_answered_ask_compiles_the_chosen_reading() {
	// no remembered answers (user 2026-10-03): each reading is written in its explicit form
	is!("3 times [0]", ints(vec![0, 0, 0]));
	is!("[1 2].map(x => x*3)", ints(vec![3, 6]));
	is!("xs=[1 2]; xs.insert(4, at: 0); xs", ints(vec![4, 1, 2]));
	is!("xs=[1 2]; xs.insert(0, at: 4); xs", ints(vec![1, 2, 0]));
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

#[test]
fn the_let_note_is_shown_until_acknowledged() {
	use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
	use warp::normalize::capture_hints;
	let let_hints = || capture_hints(|| warp::wasm_emitter::eval("let x = 3; x")).1.iter().filter(|hint| hint.original.starts_with("let ")).count();
	let path = "scratch/test_welcoming_sugar.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec!["let".to_string()]), || {
		use_acknowledgements_file(path);
		assert_eq!(let_hints(), 1);
	});
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(let_hints(), 0, "acknowledged in an earlier run");
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn a_hash_glued_to_an_expression_counts_at_line_start() {
	is!("a=[1 2 3]\n#a-1", 2);
	is!("def f(x){[x x]}\n#f(2)", 2);
	is!("x=4\n# a note\nx", 4); // `# ` with a space is a comment
	is!("x=4\n## doc comment\nx", 4);
	is!("#!/usr/bin/env warp\n3", 3);
	is!("x=4\n#include lib\nx", 4); // a directive line stays a comment
}

#[test]
fn a_spaced_double_slash_is_always_a_comment_and_says_so_once() {
	use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
	use warp::normalize::capture_hints;
	// user decision: no guessing; only glued `a//b` divides, `a // b` is a comment
	is!("items=[1 2 3 4 5]\nmid = len(items) // 2\nmid", 5);
	is!("x = 7 // 2\nx", 7);
	is!("x = 7    // 2\nx", 7);
	is!("n = 10; n //= 3\nn", 10);
	is!("items=[1 2 3 4 5]\nmid = len(items)//2\nmid", 2);
	is!("n = 10; n//=3; n", 3);
	// the note "`// …` after code is a comment" shows until acknowledged; a `//` line of its own is no news
	let notes = |code: &str| capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.canonical == "a//b").count();
	let path = "scratch/test_welcoming_sugar.slash_comment";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || assert_eq!(notes("// a note\n3"), 0));
	with_acknowledger(Acknowledging(vec!["slash-comment".to_string()]), || {
		use_acknowledgements_file(path);
		assert_eq!(notes("x = 7 // 2\nx"), 1);
	});
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(notes("x = 7 // 2\nx"), 0, "acknowledged in an earlier run");
	});
	std::fs::remove_file(path).unwrap();
}

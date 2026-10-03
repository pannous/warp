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
	use warp::diagnostic::{with_asker, ScriptedAnswers};
	let answer = |topic: &str, meaning: &str| ScriptedAnswers(vec![(topic.to_string(), meaning.to_string())]);
	with_asker(answer("list-times", "repeat the list"), || is!("[0]*3", ints(vec![0, 0, 0])));
	with_asker(answer("list-times", "multiply each element"), || is!("[1 2]*3", ints(vec![3, 6])));
	with_asker(answer("insert-order", "position first, as Python"), || is!("xs=[1 2]; xs.insert(0, 4); xs", ints(vec![4, 1, 2])));
	with_asker(answer("insert-order", "value first, as wasp"), || is!("xs=[1 2]; xs.insert(4, 0); xs", ints(vec![4, 1, 2])));
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
	use warp::diagnostic::{use_answers_file, with_asker, ScriptedAnswers, ACKNOWLEDGED};
	use warp::normalize::capture_hints;
	let let_hints = || capture_hints(|| warp::wasm_emitter::eval("let x = 3; x")).1.iter().filter(|hint| hint.original.starts_with("let ")).count();
	let path = "scratch/test_welcoming_sugar.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_asker(ScriptedAnswers(vec![("let".to_string(), ACKNOWLEDGED.to_string())]), || {
		use_answers_file(path);
		assert_eq!(let_hints(), 1);
	});
	with_asker(ScriptedAnswers(vec![]), || {
		use_answers_file(path);
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
fn a_spaced_double_slash_before_an_expression_asks_floor_or_comment() {
	use warp::diagnostic::{with_asker, with_warning_mode, ScriptedAnswers, WarningMode};
	// unanswered: an expression after `//` is taken as floor division, with a warning
	is!("items=[1 2 3 4 5]\nmid = len(items) // 2\nmid", 2);
	is!("low=3; high=8; (low + high) // 2", 5);
	is!("n = 10; n //= 3; n", 3);
	is!("x = 7 // done\nx", 7); // a lone word is taken as a comment
	is!("x = 7 // a note here\nx", 7); // prose is a comment, not asked
	is!("x = 7 // \"quoted\"\nx", 7);
	fails_with("use strict\nx = 9 // 2\nx", "is `// 2` floor division or a comment?");
	with_warning_mode(WarningMode::Error, || fails_with("x = 9 // 2", "(taking floor division)"));
	let answer = |meaning: &str| ScriptedAnswers(vec![("floor-or-comment".to_string(), meaning.to_string())]);
	with_asker(answer("a comment"), || is!("x = 9 // 2\nx", 9));
	with_asker(answer("floor division"), || is!("x = 9 // 2\nx", 4));
}

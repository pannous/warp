//! Method-style words: an unknown `.word` on a name is a loud error, the basic library words work as `x.word`, `word(x)` and `word x`
use crate::is;
use warp::wasm_emitter::eval;
use crate::common::fails_with;

/// The printed result with double quotes, whichever quote the printer uses
fn printed(code: &str) -> String {
	eval(code).serialize().replace('\'', "\"")
}

#[test]
fn test_unknown_method_word_is_an_error() {
	fails_with("x=\"hello\"; x.shout", "undefined function: shout");
	fails_with("x=\"hello\"; x.shout()", "undefined function: shout");
	fails_with("x=[1 2 3]; x.frobnicate", "undefined function: frobnicate");
}

#[test]
fn test_known_words_and_data_stay_data() {
	is!("x=[1 2 3]; x.length", 3);
	is!("x=[1 2 3]; x.count", 3);
	is!("x=[1 2 3]; x.size", 3);
	assert_eq!(printed("{a:1 b:2}"), "{a:1 b:2}");
}

#[test]
fn test_user_function_as_method_word() {
	is!("first(x):=x*2; first(5)", 10);
	is!("sum=4; sum", 4);
}

#[test]
fn test_first_and_last() {
	is!("first [1 2 3]", 1);
	is!("last [1 2 3]", 3);
	is!("x=[1 2 3]; x.first", 1);
	is!("x=[1 2 3]; x.last", 3);
	is!("x=[4 5 6]; first(x)", 4);
	is!("x=[4 5 6]; last(x)", 6);
	is!("x=\"hello\"; x.first", "h");
	is!("x=\"hello\"; x.last", "o");
}

#[test]
fn test_sum() {
	is!("sum [1 2 3]", 6);
	is!("x=[1 2 3]; x.sum", 6);
	is!("x=[1 2 3]; sum(x)", 6);
}

#[test]
fn test_reverse() {
	assert_eq!(printed("reverse [1 2 3]"), "[3 2 1]");
	assert_eq!(printed("x=[1 2 3]; x.reverse"), "[3 2 1]");
	assert_eq!(printed("x=[1 2 3]; x.reverse()"), "[3 2 1]");
	assert_eq!(printed("x=[1 2 3]; x.reverse; x"), "[1 2 3]"); // value semantics: x is unchanged
}

#[test]
fn test_sort() {
	assert_eq!(printed("sort [3 1 2]"), "[1 2 3]");
	assert_eq!(printed("x=[3 1 2]; x.sort"), "[1 2 3]");
	assert_eq!(printed("x=[3 1 2]; sort(x)"), "[1 2 3]");
	assert_eq!(printed("sort [2 2 1]"), "[1 2 2]");
}

#[test]
fn test_upper_and_lower() {
	for word in ["upper", "uppercase"] {
		assert_eq!(printed(&format!("x=\"hello\"; x.{word}")), "\"HELLO\"");
		assert_eq!(printed(&format!("x=\"hello\"; x.{word}()")), "\"HELLO\"");
		assert_eq!(printed(&format!("x=\"hello\"; {word} x")), "\"HELLO\"");
		assert_eq!(printed(&format!("{word}(\"hello\")")), "\"HELLO\"");
	}
	for word in ["lower", "lowercase"] {
		assert_eq!(printed(&format!("x=\"HeLLo\"; x.{word}")), "\"hello\"");
		assert_eq!(printed(&format!("{word} \"HeLLo\"")), "\"hello\"");
	}
	assert_eq!(printed("\"hello\".upper"), "\"HELLO\"");
}

#[test]
fn test_split() {
	assert_eq!(printed("\"a,b\".split(\",\")"), "[\"a\" \"b\"]");
	assert_eq!(printed("x=\"a,b,c\"; x.split(\",\")"), "[\"a\" \"b\" \"c\"]");
	assert_eq!(printed("split(\"a b\", \" \")"), "[\"a\" \"b\"]");
}

#[test]
fn test_join() {
	assert_eq!(printed("join [1 2] \",\""), "\"1,2\"");
	assert_eq!(printed("x=[\"a\" \"b\"]; x.join(\"-\")"), "\"a-b\"");
	assert_eq!(printed("join([1 2 3], \"\")"), "\"123\"");
}

#[test]
fn test_join_and_split_edge_cases() {
	assert_eq!(printed("join([10, -5, 300], \" \")"), "\"10 -5 300\"");
	assert_eq!(printed("join([\"x\", 7, \"yz\"], \"\")"), "\"x7yz\"");
	assert_eq!(printed("join [\"ab\" \"cd\"] \", \""), "\"ab, cd\"");
	assert_eq!(printed("\"a--b--c\".split(\"--\")"), "[\"a\" \"b\" \"c\"]");
	assert_eq!(printed("\"a,,b\".split(\",\")"), "[\"a\" \"\" \"b\"]");
	assert_eq!(printed("\"abc\".split(\",\")"), "[\"abc\"]");
}

#[test]
fn test_library_words_refuse_what_they_cannot_do() {
	fails_with("split(\"a\", \"\")", "empty separator");
	assert_eq!(printed("upper(\"é\")"), "\"É\"");
	fails_with("sort [1 \"a\"]", "not comparable"); // #26: texts sort, mixed kinds don't
	fails_with("reverse 5", "not a list");
	fails_with("join([[1], [2]], \",\")", "not a joinable item");
	fails_with("first(1, 2)", "first takes 1 argument, got 2");
	fails_with("x=[1 2]; x.first(3)", "first takes 1 argument, got 2");
	fails_with("join([1 2])", "join takes 2 arguments, got 1");
}

#[test]
fn test_replace() {
	assert_eq!(printed("\"53..7\".replace(\".\", \"0\")"), "\"53007\"");
	assert_eq!(printed("s=\"a-b-c\"; s.replace(\"-\", \"+\")"), "\"a+b+c\"");
	assert_eq!(printed("replace(\"aXbXc\", \"X\", \"\")"), "\"abc\"");
	fails_with("replace(\"a\", \"b\")", "replace takes 3 arguments, got 2");
}

#[test]
fn test_chars_of_text_literal_in_assignment() {
	assert_eq!(printed("cs=\"abc\".chars(); cs#2"), "\"b\"");
}

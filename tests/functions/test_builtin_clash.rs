// P141/P142 (notes/open_decisions.md): a definition named like a builtin that would win is a loud error at the
// definition; a library word a definition replaces (add, count, map) stays the user's own
use crate::common::fails_with;
use crate::is;

#[test]
fn an_operator_word_cannot_name_a_function() {
	fails_with("sqrt(x) := 777; sqrt(4)", "sqrt is an operator");
	fails_with("abs(x) := 777; abs(4)", "abs is an operator");
	fails_with("not(x) := 777; not(4)", "not is an operator");
	fails_with("def cbrt(x): 7\ncbrt(8)", "cbrt is an operator");
}

#[test]
fn a_cast_cannot_name_a_function() {
	fails_with("double(x) := x*2; double(21)", "double is a type");
	fails_with("def int(x): x+1\nint(5)", "int is a type");
}

// decision library-words-typed: a library word stays the library's for its own types, the program's untyped one
// takes the other values (square without `use math` is no library word here)
#[test]
fn library_words_stay_redefinable() {
	is!("add(a, b) := a*b; add(2, 3)", 6);
	is!("count(x) := 7; count([1,2])", 2);
	is!("count(x) := 7; count(3)", 7);
	is!("square(x) := 777; square(4)", 777);
}

// card def-div-error: the word after a function keyword is the name, also an infix word (`7 div 2`); it was the
// division `def div (a, b)`
#[test]
fn a_function_keyword_names_an_infix_word() {
	is!("def div(a,b) = (a - a mod b) / b; div(7,2)", 3);
	is!("fun rem(a, b) = a * b; rem(2, 3)", 6);
}

// card user-defined: a rounding word a program defines is its own, as count or map; decision library-words-typed:
// for numbers, the library's own type, it stays the library's
#[test]
fn a_defined_rounding_word_is_the_programs() {
	is!("def floor(x) := x+10; floor(2.5)", 2);
	is!("def ceil(x) := x+10; ceil 2.5", 3);
	is!("round(x) := x+10; round(2.4) * 2", 4);
	is!("def floor(x) := x+10; floor(2)", 2);
	is!("def floor(x) := x+10; floor(\"a\")", "a10");
}

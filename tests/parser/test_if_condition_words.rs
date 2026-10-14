// card parser-if-stops: a condition takes a braceless call or word with a variable argument, as an assigned value does:
// `if area certainly > 10 then` compares `area certainly`, `if square x > 5 then` compares `square x`
use crate::is;

#[test]
fn an_if_condition_takes_a_word_after_a_variable() {
	is!("area = 12 ± 1; if area certainly > 10 then \"big\" else \"small\"", "big");
	is!("area = 11 ± 2; if area certainly > 10 then \"big\" else \"small\"", "small");
	is!("area = 11 ± 2; if area possibly > 10 then \"big\" else \"small\"", "big");
}

#[test]
fn an_if_condition_takes_a_call_of_a_variable() {
	is!("square x := x*x; y = 3; if square y > 5 then 1 else 2", 1);
	is!("square x := x*x; y = 2; if square y > 5 { 1 } else { 2 }", 2);
}

#[test]
fn a_while_condition_takes_a_call_of_a_variable() {
	is!("square x := x*x; i = 0; while square i < 10 { i = i + 1 }; i", 4);
}

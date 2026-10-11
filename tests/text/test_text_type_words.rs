//! `x as str`, `String(x)`, `π as float32`: a cast takes every word of the type in BUILTIN_TYPES, aliases too, plus
//! the class spelling String (card cleanup-closed-lists)
use crate::is;

const PERSON: &str = "class person{name:text}; text(p:person) := \"P \" + p.name; x = person{name:\"a\"}; ";

#[test]
fn every_text_type_word_gives_the_text_of_an_exact_real() {
	is!("π as text", "π");
	is!("π as str", "π");
	is!("π as string", "π");
	is!("π as String", "π");
	is!("String(√2)", "√2");
	is!("str(√2)", "√2");
}

#[test]
fn every_float_type_word_rounds_an_exact_real() {
	is!("π as float", std::f64::consts::PI);
	is!("π as double", std::f64::consts::PI);
	is!("(1/4) as float32", 0.25);
	is!("(1/4) as f64", 0.25);
}

#[test]
fn every_text_type_word_calls_the_printable_operation() {
	is!(&format!("{PERSON}x as string"), "P a");
	is!(&format!("{PERSON}String(x)"), "P a");
}

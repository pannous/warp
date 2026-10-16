//! card self-lexer: a function that indexes a text parameter in a loop reads the text's characters when it is not
//! inlined (called from two places) and the text comes at run time: the array copy a list parameter gets
//! (declaration_forms::indexed_parameter_copies) held the whole text as its one item
use crate::is;

const SOURCE: &str = "samples/hello/hello.warp";

/// `first_newline(source)` called from two places, then `program_end`
fn program(program_end: &str) -> String {
	format!(
		"def first_newline(source) {{\n\
		 at = 0\n\
		 while at < source.length and source[at] != '\\n' {{ at = at + 1 }}\n\
		 at\n\
		 }}\n\
		 def through(source) := first_newline(source)\n\
		 def other(source) := first_newline(source)\n\
		 {program_end}"
	)
}

fn first_newline() -> i64 {
	let text = std::fs::read_to_string(SOURCE).unwrap();
	text.chars().position(|c| c == '\n').unwrap() as i64
}

#[test]
fn an_indexed_text_parameter_reads_characters() {
	is!(&program(&format!("first_newline(read(\"{SOURCE}\"))")), first_newline());
	is!(&program(&format!("through(read(\"{SOURCE}\"))")), first_newline());
	is!(&program(&format!("s = read(\"{SOURCE}\"); through(s)")), first_newline());
}

/// card self-parser: parser state in an object, its text read from a field (`p.code`)
#[test]
fn a_text_field_passed_on_reads_characters() {
	let state = format!("class Scanner {{ code: text }}\ns = Scanner{{code: read(\"{SOURCE}\")}}\n");
	is!(&program(&format!("{state}first_newline(s.code)")), first_newline());
	is!(&program(&format!("{state}code = s.code\nthrough(code)")), first_newline());
}

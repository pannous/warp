//! Card print-parse: code parsed at run time prints with its own operators (`:=`, `*`), not `:` for every one the
//! program itself never wrote, and joins a text with + as its literal
use crate::is;

const AREA: &str = "area(width: int, height: int) := width * height\n";

#[test]
fn run_time_parsed_code_keeps_its_operators() {
	is!(&format!("{AREA}t = area.dump + \"\"\n\"\\(parse(t))\""), "(area width:int height:int):=width*height");
	is!("t = \"a\" + \"*b\"\n\"\\(parse(t))\"", "a*b");
}

#[test]
fn a_text_plus_parsed_code_is_its_literal() {
	is!("t = \"a\" + \"*b\"\n\"x \" + parse(t)", "x a*b");
	is!("t = \"[1 \" + \"2]\"\n\"x \" + parse(t)", "x [1 2]");
	is!(
		&format!("{AREA}t = area.dump + \"\"\n\"area: \" + parse(t)"),
		"area: (area width:int height:int):=width*height"
	);
}

#[cfg(feature = "native")]
#[test]
fn print_of_a_text_and_parsed_code() {
	let printed = crate::common::printed(&format!("{AREA}print \"area:\", parse(area.dump.str)\n1"));
	assert!(printed.starts_with("area: (area width:int height:int):=width*height\n"), "{printed}");
}

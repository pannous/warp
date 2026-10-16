//! card lint-text-braces: a multi-line text is an operand, so the line that opens it with `x + "` is no continued
//! expression: `warp format` keeps the lines after the text where they were (lib/draw.warp's glsl_shader and twigl)
use warp::formatting::{format, Options};

fn formatted(code: &str) -> String {
	format(code, Options::default()).expect("it parses").text
}

#[test]
fn the_block_after_a_multiline_text_keeps_its_indentation() {
	let code = "f(x: text) := {\n\ty = x + \"\nreturn 1;\n\"\n}\nz = 1\n";
	assert_eq!(formatted(code), code);
}

#[test]
fn a_text_spliced_with_code_keeps_the_lines_after_it() {
	let code = "f(x: text) := {\n\ty = x + \"\nreturn ((\" + x + \").rgb);\n}\n\"\n\tz = y\n}\n";
	assert_eq!(formatted(code), code);
}

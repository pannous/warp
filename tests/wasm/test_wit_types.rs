use warp::warp_parser::{ParserOptions, WarpParser};

fn parse_wit(code: &str) -> warp::Node {
	WarpParser::parse_with_options(code, ParserOptions::wit())
}

#[test]
fn wit_type_arguments_close_at_the_first_angle_bracket() {
	let field = parse_wit("quotient: tuple<s64, s64>,\ncomplex: tuple<f64, f64>");
	assert_eq!(field.size(), 2);
	assert_eq!(parse_wit("block(tuple<list<node>, kind, bracket>)").size(), 2);
}

#[test]
fn wit_struct_is_a_plain_word() {
	assert_eq!(parse_wit("enum kind {\n vec,\n struct,\n other,\n}").serialize().matches("struct").count(), 1);
}

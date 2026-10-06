// Malformed Wisp gives a value, never a panic (code_quality #13): unclosed lists, unknown types, bad literals
use warp::parse_wisp;

#[test]
fn malformed_wisp_never_panics() {
	let inputs = ["(", ")", "(int", "(int x)", "(float abc)", "(char '')", "(char 'xy')", "'unterminated", "(text", "[a b",
		"(key)", "(pair x)", "(tag)", "(meta)", "(def)", "(call)", "(bool)", "(a . )", "( . b)", "(((", "]]", "(int 99999999999999999999999)",
		"(float 1e999)", "(list 1 2", "x:", ":=", "(op)", "(typed)", "ø ø (", "(nil x y)", "\"", "(text \"a)", "(unknown 1 2)"];
	for input in inputs {
		let parsed = std::panic::catch_unwind(|| parse_wisp(input));
		assert!(parsed.is_ok(), "parse_wisp({input:?}) panicked");
	}
}

#[test]
fn malformed_wisp_is_an_error_not_a_repaired_value() {
	for input in ["(", "(int x)", "(float abc)", "(text", "[a b", "\"open"] {
		assert!(matches!(parse_wisp(input), warp::Node::Error(_)), "parse_wisp({input:?}) = {:?}", parse_wisp(input));
	}
	assert_eq!(parse_wisp("(int 99999999999999999999999)").serialize(), "99999999999999999999999");
}

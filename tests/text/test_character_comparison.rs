use warp::is;

// samples/calculator.wasp, samples/json_parser.wasp: a character held in a variable compares by code point
// (was "not an int": the variable was unboxed as an Int)

#[test]
fn test_character_variable_compares_with_character_literal() {
	is!("c='5'; c >= '0'", true);
	is!("c='5'; c >= '0' and c <= '9'", true);
	is!("c='x'; c >= '0' and c <= '9'", false);
	is!("c='q'; (c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z')", true);
}

#[test]
fn test_character_element_of_text_compares() {
	is!("s=\"a5\"; c=s#2; c >= '0' and c <= '9'", true);
}

#[test]
fn test_character_test_words() {
	is!("c='5'; c.is_digit()", true);
	is!("'x'.is_digit()", false);
	is!("c='q'; c.is_alpha()", true);
	is!("c='_'; c.is_alphanumeric()", false);
	is!("c='7'; c.is_alphanumeric()", true);
}

#[test] // the cast of a one-character text held in a variable reads its digit, like "12" (not its code point)
fn test_one_character_text_as_int() {
	is!("x=\"5\"; x as int", 5);
	is!("x=\"a\"; try (x as int) + 1 else 0", 0);
}

#[test] // user 2026-10-03: "obviously one of five is five": int of a digit character is its digit
fn test_int_of_character_is_its_digit() {
	is!("int('5')", 5);
	is!("c='5'; int(c)", 5);
	is!("s=\"a5\"; int(s#2)", 5);
	is!("int('5')+1", 6);
}

#[test] // user 2026-10-03: "use ord ordinal codepoint() to get the code point"
fn test_ord_is_the_code_point() {
	is!("ord('A')", 65);
	is!("ordinal('a')", 97);
	is!("codepoint('5')", 53);
	is!("c='x'; ord(c)", 120);
	is!("x=65 as char; ord(x)", 65);
	is!("'A'.ord()", 65);
	is!("try ord(\"ab\") else 0", 0);
}

#[test] // a character that is no digit is no number: its code point is ord(c)
fn test_non_digit_character_as_int_is_no_number() {
	is!("try int('a') else 0", 0);
	is!("try ('A' as int) else 0", 0);
	is!("x=65 as char; try (x as int) else 0", 0);
	is!("'7' as int", 7);
}

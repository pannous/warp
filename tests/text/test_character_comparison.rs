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

//! Card print-effects-walked: a reflection result printed directly stays one value instead of becoming a character walk.

#[cfg(feature = "native")]
fn printed_lines(code: &str) -> Vec<String> {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(String::from).collect()
}

#[cfg(feature = "native")]
#[test]
fn print_of_property_prints_one_value() {
	assert_eq!(printed_lines("f(x) := print x\nprint effects of f"), ["IO"]);
	assert_eq!(printed_lines("p = {name: \"Ada\"}\nprint name of p"), ["Ada"]);
}

use warp::is;
use warp::wasm_emitter::eval;

fn fails_loudly(code: &str) {
	let result = format!("{:?}", eval(code));
	assert!(result.contains("Error"), "{code}: {result}");
}

#[test]
fn texts_concatenate_with_plus() {
	is!("\"ab\"+\"cd\"", "abcd");
	is!("'Hello, ' + 'World!'", "Hello, World!");
	is!("\"a\"+\"b\"", "ab");
	is!("\"a\"+\"bc\"", "abc");
	is!("\"ab\"+\"c\"+\"d\"", "abcd");
	is!("\"\"+\"x\"", "x");
}

#[test]
fn text_variables_and_compound_assignment() {
	is!("x=\"hello\"; y=\" world\"; x+y", "hello world");
	is!("x=\"hello\"; x+=\" world\"; x", "hello world");
	is!("x=\"a\"; x+=\"b\"; x", "ab");
	is!("x=\"\"; x+=\"a\"; x+=\"b\"; x+=\"c\"; x", "abc");
}

#[test]
fn concatenation_takes_results_of_functions_and_library_words() {
	is!("\"a\".upper + \"b\"", "Ab");
	is!("\"a\" + \"b\".upper", "aB");
	is!("f(x):=x+\"!\"; f(\"hi\")", "hi!");
}

#[test]
fn text_plus_number_stays_an_error() {
	fails_loudly("\"a\"+1");
	fails_loudly("1+\"a\"");
	fails_loudly("\"ab\"+1");
}

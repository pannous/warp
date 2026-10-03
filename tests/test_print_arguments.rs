// `print` with several arguments writes them separated by a space, like Python: `print(a, b)` and `print a, b`.
// The call is worth the printed text.
use warp::is;

#[cfg(feature = "native")] // runs the warp binary
fn printed(code: &str) -> String {
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_warp")).args(["--no-ask", code]).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
#[cfg(feature = "native")]
fn print_joins_its_arguments_with_a_space() {
	assert!(printed("g=\"hi\"; print(g, g)").starts_with("hi hi\n"));
	assert!(printed("g=\"hi\"; print g, g").starts_with("hi hi\n"));
	assert!(printed("print(\"x\", 1, 2.5)").starts_with("x 1 2.5\n"));
	assert!(printed("n=3; print \"n =\", n").starts_with("n = 3\n"));
}

#[test]
fn print_of_several_arguments_is_worth_the_printed_text() {
	is!("g=\"hi\"; print(g, g)", "hi hi");
	is!("g=\"hi\"; print g, g", "hi hi");
	is!("n=3; print(\"n\", n)", "n 3");
}

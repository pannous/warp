// Swift argument labels: `func greet(person name: String)` is called `greet(person: "Bob")` and reads `name` in its
// body; `_ name` takes the argument without a label
use crate::is;

#[test]
fn a_label_names_the_argument_and_the_name_is_read_inside() {
	is!("func greet(person name: String) -> String { \"Hi \" + name }; greet(person: \"Bob\")", "Hi Bob");
	is!("func move(from a: Int, to b: Int) -> Int { b - a }; move(from: 2, to: 9)", 7);
	is!("func move(from a: Int, to b: Int) -> Int { b - a }; move(2, 9)", 7);
}

#[test]
fn an_underscore_label_takes_the_argument_without_a_name() {
	is!("func twice(_ x: Int) -> Int { x * 2 }; twice(4)", 8);
	is!("func add(_ a: Int, _ b: Int) -> Int { a + b }; add(2, 3)", 5);
}

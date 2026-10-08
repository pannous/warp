//! Page tests written in warp (card web-testing, src/page_tests.rs): `test "…" { render …; click "…"; check … }` runs
//! on a headless page; the program's value counts the passed tests or names the failed ones
use warp::node::Node;
use warp::wasm_emitter::eval;

const COUNTER: &str = "def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"Add\" } p{ \"n \" + count } }\n}\n";

#[test]
fn passing_page_tests_are_counted() {
	let tests = "test \"counter\" {\n\trender Counter(1)\n\tclick \"Add\"\n\tcheck text is \"Addn 2\"\n}\ntest \"form\" {\n\tname = \"\"\n\trender div{ input{ placeholder: \"name\" bind: name } p{ \"hi \" + name } }\n\tfill \"name\" with \"Ada\"\n\tcheck html.ends_with(\"<p>hi Ada</p></div>\")\n}\n";
	assert_eq!(eval(&format!("{COUNTER}{tests}")), Node::Text("2 tests passed".to_string()));
}

#[test]
fn a_failing_check_names_its_test_and_the_page() {
	let outcome = eval(&format!("{COUNTER}test \"wrong\" {{ render Counter(0); check text is \"Addn 1\" }}"));
	let Node::Error(problem) = &outcome else { panic!("expected an error, got {}", outcome.serialize()) };
	assert!(problem.name().contains("test \"wrong\": check text is \"Addn 1\" gave no; the page shows \"Addn 0\""), "{}", problem.name());
}

#[test]
fn a_missing_button_fails_the_test() {
	let outcome = eval(&format!("{COUNTER}test \"missing\" {{ render Counter(0); click \"Remove\" }}"));
	assert!(matches!(&outcome, Node::Error(problem) if problem.serialize().contains("no element showing")), "{}", outcome.serialize());
}

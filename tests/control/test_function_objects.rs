//! Function objects (card module-methods, decision reflection): `methods(math)`, `math.methods` and `methods of math`
//! are the module's functions themselves; every function has .name .signature .return_type .parameters .doc
use crate::is;

const TWICE: &str = "# doubles a number\ntwice(x) := x*2\n";

#[test]
fn a_modules_methods_are_its_functions() {
	is!("use math\nfs = methods(math)\nfs#1(12, 18)", 6);
	is!("use math\ncount(math.methods) == count(dir(math))", true);
	is!("use math\ncount(methods of math) == count(dir(math))", true);
}

#[test]
fn a_function_knows_its_name_and_return_type() {
	is!(&format!("{TWICE}twice.name"), "twice");
	is!(&format!("{TWICE}twice.return_type"), "int");
	is!("use math\ngcd.name", "gcd");
	is!("use math\ngcd.return_type", "int");
}

#[test]
fn the_words_of_a_function_reference() {
	is!(&format!("{TWICE}(function twice).doc"), "doubles a number");
	is!(&format!("{TWICE}(function twice).name"), "twice");
}

#[test]
fn a_function_value_answers_at_run_time() {
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.doc"), "doubles a number");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.name"), "twice");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.signature"), "(x) -> int");
}

#[test]
#[cfg(feature = "native")]
fn a_printed_function_shows_its_doc_and_definition() {
	let printed = crate::common::printed(&format!("{TWICE}print(function twice)"));
	assert_eq!(printed.trim(), "# doubles a number\ntwice(x) := x*2");
}

#[test]
fn only_the_comment_directly_above_is_the_doc() {
	is!("# header\n\n# doubles\ntwice(x) := x*2\ntwice.doc", "doubles");
	is!("use math\ngcd.doc", "");
	is!("use math\nlerp.doc", "the value a fraction t of the way from a to b");
}

#[test]
#[cfg(feature = "native")]
fn a_function_in_a_list_prints_by_its_name() {
	let printed = crate::common::printed(&format!("{TWICE}print [function twice]"));
	assert_eq!(printed.trim(), "[twice]");
}

#[test]
fn printing_a_function_without_parameters_calls_it_and_warns() {
	crate::common::warns_with("hi() := \"hello\"\nprint hi\n1", 1, "write `print function hi`");
}

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

#[test]
fn a_classes_methods_are_its_functions() {
	const DOG: &str = "class Dog { name: text; bark() := \"wuff \" + name }\nd = Dog(\"rex\")\n";
	is!(&format!("{DOG}fs = d.methods\nfs#1(d)"), "wuff rex");
	is!(&format!("{DOG}[m.name for m in methods(Dog)]"), warp::texts(vec!["bark"]));
}

#[test]
fn the_type_of_a_function_names_its_signature() {
	is!("use math\ntype(gcd)", "function: (a:int, b:int) -> int");
	is!("use math\n(function gcd) is function", true);
	is!("type(sqrt)", "builtin: (x:number) -> number"); // decision dir-lists
}

#[test]
fn a_values_methods_are_its_types_functions() {
	is!("fs = methods(\"hi\")\nfs#1.name", "repeat");
	is!("s = \"hi\"\ncount(s.methods) == count(dir(s))", true);
	is!("m = {methods: 3}\nm.methods", 3);
}

// other names of a function's words (user): read as the canonical word, with a note naming it
#[test]
fn description_is_doc() {
	is!(&format!("{TWICE}twice.description"), "doubles a number");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.description"), "doubles a number");
}

#[test]
fn returns_is_return_type() {
	is!(&format!("{TWICE}twice.returns"), "int");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.returns"), "int");
}

#[test]
fn representation_is_dump() {
	is!(&format!("{TWICE}twice.representation"), "twice(x) := x*2");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.representation"), "twice(x) := x*2");
}

// `f.full`: what `print f` shows
#[test]
fn full_is_the_doc_and_the_definition() {
	is!(&format!("{TWICE}twice.full"), "# doubles a number\ntwice(x) := x*2");
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.full"), "# doubles a number\ntwice(x) := x*2");
}

#[test]
fn a_maps_own_description_stays_its_value() {
	is!("m = {description: 3}\nm.description", 3);
}

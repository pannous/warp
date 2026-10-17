//! card name-after: ` /name` after a word is a path wherever the program defines that word as a function, before or
//! after the call, with := or def, and after a variable holding a function; juxtaposed paths stay separate values
use crate::is;

#[test]
fn a_path_after_a_function_defined_later() {
	is!("show /tmp\nshow(p) := p", "/tmp");
	is!("show /tmp\ndef show(p){p}", "/tmp");
	is!("def show(p){p}\nshow /tmp", "/tmp");
}

#[test]
fn a_path_after_a_variable_holding_a_function() {
	is!("f = x => x; f /tmp", "/tmp");
}

#[cfg(feature = "native")]
#[test]
fn juxtaposed_paths_print_as_separate_values() {
	assert_eq!(crate::common::printed("print /a /b").lines().next(), Some("/a /b"));
	assert_eq!(crate::common::printed("print \"ab\" \"c\"").lines().next(), Some("abc"));
}

/// a name a definition binds as a parameter divides, also when a program function has that name (reviewer)
#[test]
fn a_parameter_shadowing_a_function_divides() {
	is!("scale(x) := x * 2\ndef area(scale, h) { scale /h }\narea(6, 2)", 3);
	is!("scale(x) := x * 2\narea(scale, h) := scale /h\narea(6, 2)", 3);
	is!("xs = [1 2]\ntotal = sum(xs.map(x => x))\ntotal /2", 1.5);
}

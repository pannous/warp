//! card path-parameter-scope: a parameter is a variable only inside its own definition, so ` /name` after a function's
//! name is a path everywhere else, also when some other definition takes a parameter of that name
use crate::is;

#[test]
fn a_parameter_divides_only_inside_its_definition() {
	is!("show(p) := p\ndef g(show) := show + 1\nshow /tmp", "/tmp");
	is!("show(p) := p\ng(show) := show /2\nshow /tmp", "/tmp");
	is!("show(p) := p\ng = show => show /2\ng(6)", 3);
	is!("show(p) := p\ndef area(show, h) { show /h }\narea(6, 2)", 3);
}

#[test]
fn a_definitions_parameter_scope_ends_with_it() {
	is!("show(p) := p\ndef g(show) { show + 1 }\nx = show /tmp\nx", "/tmp");
}

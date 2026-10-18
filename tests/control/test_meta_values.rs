//! `meta(x)` (decision reflection-plain): what the compiler knows of a module or a class, as a map, also of a value
//! known only at run time; `dir(m)` of a module is its meta's words
use crate::is;

const CLASSES: &str = "class A { a: int }\nclass P extends A { x: int }\nlength(p: P) := p.x\n";

#[test]
fn a_modules_meta() {
	is!("use math\nmeta(math).name", "math");
	is!("use math\nmeta(math).words == dir(math)", true);
}

#[test]
fn a_classes_meta() {
	is!(&format!("{CLASSES}meta(P).fields == [\"a\" \"x\"]"), true);
	is!(&format!("{CLASSES}meta(P).methods#1.name"), "length");
	is!(&format!("{CLASSES}meta(P).parent"), "A");
}

#[test]
fn meta_of_a_value_known_at_run_time() {
	is!(&format!("{CLASSES}meta(type(P(1, 2))).fields == [\"a\" \"x\"]"), true);
}

#[test]
#[ignore = "card regression-module: a module symbol through two untyped functions reads as an int"]
fn a_module_value_through_a_function() {
	is!("use math\nf(m) := meta(m).name\nf(math)", "math");
}

#[test]
fn dir_of_a_module_known_at_run_time() {
	is!("use math\ng(m) := dir(m)\ng(math) == dir(math)", true);
}

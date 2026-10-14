// card param-map-pattern: a map pattern is a parameter as a list or class pattern is, `greet({name}) := "hi " + name`;
// card param-pattern-local-warning: the names a pattern parameter binds are the function's own, no main-level question
use crate::common::{fails_with, warnings_of};
use crate::is;
use warp::Node;

const POINT: &str = "class Point { x: int; y: int }";

#[test]
fn a_map_parameter_names_its_fields() {
	is!("greet({name}) := \"hi \" + name\ngreet({name: \"Ann\"})", "hi Ann");
	is!("area({width, height}) := width * height\narea({width: 3, height: 4, unit: \"cm\"})", 12);
	is!("label({name: n}) := \"#\" + n\nlabel({name: \"x\"})", "#x");
}

#[test]
fn a_map_without_the_field_is_an_error() {
	fails_with("greet({name}) := \"hi \" + name\ngreet({age: 3})", "no case");
}

#[test]
fn pattern_names_are_no_main_level_variables() {
	asks_nothing(
		&format!("{POINT}\ntotal(Point{{x, y}}) := x + y\nx = 10\ntotal(Point{{x: 3, y: 4}}) + x"),
		Node::from(17),
	);
	asks_nothing(
		"greet({name}) := \"hi \" + name\nname = \"Bo\"\ngreet({name: \"Ann\"}) + name",
		Node::from("hi AnnBo"),
	);
}

/// `code` gives `expected` without asking whether a pattern name is the main-level variable
fn asks_nothing(code: &str, expected: Node) {
	let (value, warnings) = warnings_of(code);
	assert_eq!(value, expected, "{code}");
	assert!(!warnings.iter().any(|warning| warning.contains("main-level")), "{warnings:?}");
}

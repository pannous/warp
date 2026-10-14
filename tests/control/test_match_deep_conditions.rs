//! Conditions at any depth of a pattern (card deep-conditions, step 2 of tree-patterns): a field or an item takes what
//! a whole arm takes, a comparison `>= 18`, a range `13 to 17`, a type `text` and alternatives `1 | 2`
use crate::is;

const PERSON: &str = "classify(p) := match p { \
	{body: {user: {age: >= 18, name: text}}} => \"adult\"; \
	{body: {user: {age: 13 to 17}}} => \"teen\"; \
	_ => \"child\" }; ";

#[test]
fn comparisons_and_types_inside_maps() {
	is!(&format!("{PERSON}classify({{body: {{user: {{name: \"Ann\", age: 31}}}}}})"), "adult");
	is!(&format!("{PERSON}classify({{body: {{user: {{name: 7, age: 31}}}}}})"), "child");
	is!(&format!("{PERSON}classify({{body: {{user: {{name: \"Bo\", age: 15}}}}}})"), "teen");
	is!(&format!("{PERSON}classify({{body: {{user: {{name: \"Cy\", age: 3}}}}}})"), "child");
}

#[test]
fn alternatives_inside_maps() {
	is!("match {code: 301} { {code: 301 | 302} => \"moved\"; _ => \"other\" }", "moved");
	is!("match {code: 404} { {code: 301 | 302} => \"moved\"; _ => \"other\" }", "other");
}

#[test]
fn conditions_inside_lists() {
	is!("match [5, \"x\"] { [> 3, text] => \"big then text\"; _ => \"other\" }", "big then text");
	is!("match [2, \"x\"] { [> 3, text] => \"big then text\"; _ => \"other\" }", "other");
	is!("match [15, 2] { [10 to 20, n] => n; _ => 0 }", 2);
}

#[test]
fn is_with_a_condition_inside_a_map() {
	is!("x = {name: \"Ann\", age: 31}; if x is {age: >= 18} then \"adult\" else \"minor\"", "adult");
	is!("x = {name: \"Bo\", age: 12}; if x is {age: >= 18} then \"adult\" else \"minor\"", "minor");
}

#[test]
fn nested_alternatives_bind_no_names() {
	crate::common::fails_with("match {a: 1} { {a: x | 2} => x; _ => 0 }", "alternatives bind no names");
}

//! Map arms `{name: n} =>` match a map partially: each named field must be there and match its pattern, fields not
//! named are ignored, `{name}` binds the field to its name, and maps inside match by shape at any depth (card
//! map-patterns, step 1 of tree-patterns)
use crate::is;

const RESPONSE: &str = "response = {status: 200, body: {user: {name: \"Ann\", email: \"ann@x.org\", age: 31}}}; ";

#[test]
fn map_arm_binds_a_field() {
	is!("match {name: \"Ann\", age: 3} { {name: n} => n }", "Ann");
	is!("match {name: \"Ann\", age: 3} { {age} => age + 1 }", 4);
}

#[test]
fn map_arm_ignores_fields_not_named() {
	is!("match {name: \"Ann\", age: 3} { {name: \"Ann\"} => 1; _ => 2 }", 1);
	is!("match {name: \"Bo\", age: 3} { {name: \"Ann\"} => 1; _ => 2 }", 2);
}

#[test]
fn map_arm_needs_its_fields() {
	is!("match {name: \"Ann\"} { {age} => age; {name} => name }", "Ann");
	is!("match 3 { {name} => name; _ => \"no map\" }", "no map");
}

#[test]
fn map_arms_match_at_any_depth() {
	let arms = "{status: 200, body: {user: {name, email}}} => name + \" <\" + email + \">\"; \
		{status: 404} => \"missing\"; {error: message} => message";
	is!(&format!("{RESPONSE}match response {{ {arms} }}"), "Ann <ann@x.org>");
	is!(&format!("response = {{status: 404}}; match response {{ {arms} }}"), "missing");
	is!(&format!("response = {{error: \"down\"}}; match response {{ {arms} }}"), "down");
}

#[test]
fn is_with_a_map_pattern_binds_its_names() {
	is!("x = {name: \"Ann\", age: 3}; if x is {age} then age else 0", 3);
	is!("x = {name: \"Ann\", age: 3}; if x is {size} then size else 0", 0);
	is!(&format!("{RESPONSE}if response is {{body: {{user: {{name}}}}}} then name else \"\""), "Ann");
}

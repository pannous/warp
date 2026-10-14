//! card member-completion: `xs = [1 2]; xs.` offers the list's methods, in the playground editor and the console REPL;
//! both ask warp::members::member_names with the code before the dot
use warp::members::{member_names, LIST_MEMBERS, MAP_MEMBERS, NUMBER_MEMBERS, TEXT_MEMBERS};

fn offers(code: &str, expected: &[&str]) {
	let names = member_names(code);
	for name in expected {
		assert!(names.iter().any(|offered| offered == name), "`{code}.` should offer {name}, offered {names:?}");
	}
}

fn offers_none_of(code: &str, unexpected: &[&str]) {
	let names = member_names(code);
	for name in unexpected {
		assert!(!names.iter().any(|offered| offered == name), "`{code}.` should not offer {name}, offered {names:?}");
	}
}

#[test]
fn a_list_variable_offers_list_methods() {
	offers("xs = [1 2]; xs", &["count", "sum", "map", "filter", "sort", "first", "unique", "zip"]);
	offers_none_of("xs = [1 2]; xs", &["upper", "keys"]);
}

#[test]
fn literals_offer_their_type_s_methods() {
	offers("[1 2 3]", &["count", "map"]);
	offers("\"hello\"", &["upper", "split", "capitalize"]);
	offers("print 1; x = 2.5\nx", &["string", "int"]);
}

#[test]
fn a_text_variable_offers_text_methods() {
	offers("s = \"abc\"\nprint s", &["upper", "lower", "trim", "replace"]);
	offers_none_of("s = \"abc\"\nprint s", &["sum", "map"]);
}

#[test]
fn a_map_offers_its_keys_first() {
	let names = member_names("m = {name: \"Ann\", age: 3}; m");
	assert_eq!(names[..2], ["name", "age"]);
	offers("m = {name: \"Ann\", age: 3}; m", &["keys", "values", "get"]);
}

#[test]
fn an_instance_offers_its_fields_and_methods() {
	let code = "class Point{x: int, y: int\nlength() := x + y}\np = Point(1, 2)\np";
	let names = member_names(code);
	assert_eq!(names[..3], ["x", "y", "length"]);
}

#[test]
fn a_standard_module_offers_its_words() {
	offers(
		"use time\ntime",
		&warp::modules::std_module_definitions("time").unwrap().iter().map(String::as_str).collect::<Vec<_>>(),
	);
}

#[test]
fn a_chain_and_a_call_receiver_are_one_receiver() {
	offers("xs = [3 1 2]; xs.sort()", &["count", "map"]);
	offers("words = \"a b\".split(\" \")\nwords", &["count", "join"]);
}

#[test]
fn an_unknown_receiver_offers_nothing() {
	assert!(member_names("nobody").is_empty());
	assert!(member_names("").is_empty());
}

/// Arguments a method may need to be found at all: `xs.map()` is "undefined function: map", `xs.map(x => x)` is map
const ARGUMENT_FORMS: [&str; 3] = ["", "x => x", "0, (a, b) => a"];

/// Every offered builtin works in the dot form on a value of its type: no "undefined function" for any of them
#[test]
fn every_offered_builtin_is_a_method_of_its_type() {
	let receivers = [
		("[3 1 2]", &LIST_MEMBERS[..]),
		("\"abc\"", &TEXT_MEMBERS[..]),
		("{a: 1}", &MAP_MEMBERS[..]),
		("2.5", &NUMBER_MEMBERS[..]),
	];
	for (receiver, members) in receivers {
		for member in members {
			let failures: Vec<String> = ARGUMENT_FORMS
				.iter()
				.map(|arguments| {
					warp::wasm_emitter::eval(&format!("v = {receiver}\nv.{member}({arguments})")).serialize()
				})
				.collect();
			let unknown = |shown: &String| shown.contains("undefined function") || shown.contains("no field");
			assert!(!failures.iter().all(unknown), "{receiver}.{member}: {failures:?}");
		}
	}
}

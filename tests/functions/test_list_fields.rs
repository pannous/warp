//! Card people-map (user): a field read on a list of instances reads it of each, `people's name` is
//! `people.map(p => p.name)`; on one instance it stays the field, a list's own words (`people.count`) stay the list's
use warp::wasm_emitter::eval;

const PEOPLE: &str = "class Person { name: text; age: int }; people = [Person{name: \"Ann\", age: 30}, Person{name: \"Bob\", age: 20}]; ";

fn printed(code: &str) -> String {
	eval(&format!("{PEOPLE}{code}")).serialize()
}

#[test]
fn test_possessive_on_a_list_maps_the_field() {
	assert_eq!(printed("people's name"), printed("people.map(p => p.name)"));
	assert_eq!(printed("people's name"), "[\"Ann\" \"Bob\"]");
	assert_eq!(printed("people's age"), "[30 20]");
}

#[test]
fn test_dot_on_a_list_maps_the_field() {
	assert_eq!(printed("people.name"), "[\"Ann\" \"Bob\"]");
	assert_eq!(printed("name of people"), "[\"Ann\" \"Bob\"]");
}

#[test]
fn test_one_instance_and_list_words_stay() {
	assert_eq!(printed("first = people#1; first's name"), "\"Ann\"");
	assert_eq!(printed("people.count"), "2");
}

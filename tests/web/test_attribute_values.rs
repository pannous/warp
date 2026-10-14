//! card attribute-values: an attribute value of the wrong kind warns where it is written: a text for a boolean
//! attribute (`checked: "no"` leaves it out, as `checked: "yes"` does), a word for a number attribute, a number for an
//! address
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings().into_iter().map(|warning| warning.message).collect()
}

fn warns(code: &str, expected: &str) {
	let warned = warnings_of(code);
	assert!(warned.iter().any(|warning| warning == expected), "{code}: {warned:?}");
}

#[test]
fn a_text_for_a_boolean_attribute_warns() {
	warns(
		"input{ type: \"checkbox\" aria-label: \"a\" checked: \"no\" }",
		"checked takes yes or no: the text \"no\" leaves it out",
	);
	warns("button{ disabled: \"yes\" \"b\" }", "disabled takes yes or no: the text \"yes\" leaves it out");
}

#[test]
fn a_word_for_a_number_attribute_warns() {
	warns("textarea{ aria-label: \"t\" rows: \"many\" }", "rows takes a number, not \"many\"");
}

#[test]
fn a_number_for_an_address_warns() {
	warns("a{ href: 3 \"go\" }", "href takes an address, not the number 3");
}

#[test]
fn values_of_the_right_kind_stay_quiet() {
	for code in [
		"input{ type: \"checkbox\" aria-label: \"a\" checked: yes }",
		"done = no\ninput{ type: \"checkbox\" aria-label: \"a\" checked: done }",
		"textarea{ aria-label: \"t\" rows: 3 cols: \"40\" }",
		"a{ href: \"/x\" \"go\" }",
		"p{ id: 5 \"x\" }",
	] {
		let mistakes: Vec<String> = warnings_of(code).into_iter().filter(|warning| warning.contains("takes")).collect();
		assert_eq!(mistakes, Vec::<String>::new(), "{code}");
	}
}

// card warn-step-any: a number attribute's keyword (`step: "any"`) and dates (`min: "2024-01-01"`) are valid HTML
#[test]
fn a_number_attribute_takes_its_keyword_and_dates() {
	let warned = warnings_of("input{ type: \"number\" aria-label: \"x\" step: \"any\" min: \"2024-01-01\" }");
	assert!(warned.iter().all(|warning| !warning.contains("takes a number")), "{warned:?}");
}

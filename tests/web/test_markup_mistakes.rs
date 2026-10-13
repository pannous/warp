//! Is our HTML type safe (card our-html): markup that would render wrong is warned about where it is written, with the
//! near name as its fix: a key no attribute (shown as text), a handler of no event, an input type the browser does not
//! know, content of an element that takes none, a block under a name no element
use warp::diagnostic::take_warnings;
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
fn an_unknown_attribute_warns() {
	warns(
		"a{ hreff: \"/x\" \"go\" }",
		"hreff is no attribute of a: it shows as text",
	);
	warns(
		"div{ colour: \"red\" \"hi\" }",
		"colour is no attribute of div: it shows as text",
	);
	warns(
		"p{ onclick: \"go()\" \"x\" }",
		"onclick is no attribute of p: it shows as text",
	);
}

#[test]
fn an_unknown_event_warns() {
	warns(
		"x = 0\nbutton{ on clik { x = 1 } \"b\" }",
		"clik is no event of button: the handler shows as text",
	);
}

#[test]
fn an_unknown_input_type_warns() {
	warns(
		"input{ type: \"nubmer\" aria-label: \"n\" }",
		"nubmer is no input type: the browser shows a text field",
	);
}

// card unknown-tag: a block under a name no element is the text `divv:"hi"`
#[test]
fn an_unknown_tag_among_markup_warns() {
	warns("div{ divv{ \"hi\" } }", "divv is no element: it shows as text");
	warns("ul{ lli{ \"x\" } }", "lli is no element: it shows as text");
	// said once, as written: the run's markup holds it as the attribute divv: "hi"
	warp::diagnostic::take_runtime_warnings();
	let warned = warnings_of("div{ divv{ \"hi\" } }");
	let at_run_time = warp::diagnostic::take_runtime_warnings();
	assert!(
		!warned
			.iter()
			.chain(&at_run_time)
			.any(|warning| warning.contains("no attribute")),
		"{warned:?} {at_run_time:?}"
	);
}

#[test]
fn content_of_a_void_element_warns() {
	warns(
		"img{ alt: \"a\" \"text inside\" }",
		"img takes no content: it is dropped",
	);
}

#[test]
fn right_markup_stays_quiet() {
	for code in [
		"a{ href: \"/x\" \"go\" }",
		"div{ class: \"c\" id: \"d\" data-x: 1 aria-label: \"l\" key: 3 \"hi\" }",
		"x = 0\nbutton{ on click { x = 1 } \"b\" }",
		"name = \"\"\ninput{ type: \"email\" bind: name aria-label: \"n\" }",
		"img{ src: \"a.png\" alt: \"a\" }",
		"li{ transition: fade 200ms \"x\" }",
		"div{ my-card{ \"x\" } }",
		"card(t) := p{ t }\ndiv{ card{ \"x\" } }",
	] {
		assert_eq!(warnings_of(code), Vec::<String>::new(), "{code}");
	}
}

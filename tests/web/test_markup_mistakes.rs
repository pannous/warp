//! Is our HTML type safe (card our-html): markup that would render wrong is warned about where it is written, with the
//! near name as its fix: a key no attribute (shown as text), a handler of no event, an input type the browser does not
//! know, content of an element that takes none, a block under a name no element
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
fn an_unknown_attribute_warns() {
	warns("a{ hreff: \"/x\" \"go\" }", "hreff is no attribute of a: it shows as text");
	warns("div{ colour: \"red\" \"hi\" }", "colour is no attribute of div: it shows as text");
	warns("p{ onclick: \"go()\" \"x\" }", "onclick is no attribute of p: it shows as text");
}

#[test]
fn an_unknown_event_warns() {
	warns("x = 0\nbutton{ on clik { x = 1 } \"b\" }", "clik is no event of button: the handler shows as text");
}

#[test]
fn an_unknown_input_type_warns() {
	warns("input{ type: \"nubmer\" aria-label: \"n\" }", "nubmer is no input type: the browser shows a text field");
}

// card unknown-tag: a block under a name no element is the text `divv:"hi"`
#[test]
fn an_unknown_tag_among_markup_warns() {
	warns("div{ divv{ \"hi\" } }", "divv is no element: it shows as text");
	warns("ul{ lli{ \"x\" } }", "lli is no element: it shows as text");
	// said once, as written: the run's markup holds it as the attribute divv: "hi"
	warp::errors::diagnostic::take_runtime_warnings();
	let warned = warnings_of("div{ divv{ \"hi\" } }");
	let at_run_time = warp::errors::diagnostic::take_runtime_warnings();
	assert!(
		!warned.iter().chain(&at_run_time).any(|warning| warning.contains("no attribute")),
		"{warned:?} {at_run_time:?}"
	);
}

#[test]
fn content_of_a_void_element_warns() {
	warns("img{ alt: \"a\" \"text inside\" }", "img takes no content: it is dropped");
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

/// card class-literal: the fields of a class literal are no elements, even named like one (`source`, `title`, `label`)
#[test]
fn a_class_literal_field_named_like_an_element_is_no_markup() {
	let code = "class Parser { source: text }\np = Parser{source: \"ab\"}\np.source";
	assert_eq!(warnings_of(code), Vec::<String>::new(), "{code}");
	crate::is!(code, "ab");
	let nested = "class Page { title: text }\nPage{title: \"T\"}.title";
	assert_eq!(warnings_of(nested), Vec::<String>::new(), "{nested}");
	// a field's value is still markup
	warns(
		"class Page { body: any }\npage = Page{body: img{ alt: \"a\" \"text inside\" }}",
		"img takes no content: it is dropped",
	);
}

/// card html-meta-warnings (decision html-meta): the page facts of html, `description:` and `meta: {author: …}`, go into
/// the head without a warning; a meta element's own attributes (`charset`) are still checked as attributes
#[test]
fn page_facts_are_no_attributes() {
	let code = "html{ lang: \"en\" description: \"d\" meta: {author: \"Ann\"} }";
	warp::errors::diagnostic::take_runtime_warnings();
	assert_eq!(warnings_of(code), Vec::<String>::new(), "{code}");
	let at_run_time = warp::errors::diagnostic::take_runtime_warnings();
	assert!(at_run_time.is_empty(), "{at_run_time:?}");
	let html = warp::web::markup::to_html(&eval(code));
	assert!(html.contains("<meta name=\"author\" content=\"Ann\">"), "{html}");
	warns(
		"html{ head{ meta{ charset: \"utf-8\" contnt: \"x\" } } }",
		"contnt is no attribute of meta: it shows as text",
	);
}

/// cards data-map, body-key-markup: a data map's keys are no elements, even named like one (`body`, `title`), when it
/// has a field named like no element; a field's value is still markup
#[test]
fn a_data_map_key_named_like_an_element_is_no_markup() {
	let code = "{status: 200, body: {user: {name: \"Ann\"}}}";
	assert_eq!(warnings_of(code), Vec::<String>::new(), "{code}");
	let assigned = "r = {status: 200, body: {user: 1}}\nr.body.user";
	assert_eq!(warnings_of(assigned), Vec::<String>::new(), "{assigned}");
	warns("r = {status: 200, page: img{ alt: \"a\" \"text inside\" }}", "img takes no content: it is dropped");
}

/// card data-map: a map pattern whose element-named key holds a map with a plain key is data too
#[test]
fn a_nested_data_map_is_no_markup() {
	let code = "response = {status: 200, body: {user: {age: 31}}}\nif response is {body: {user: {age}}} then age";
	assert_eq!(warnings_of(code), Vec::<String>::new(), "{code}");
}

/// card dash-element (decision unknown-tag): a custom element is named with a dash and renders as one, without a warning
#[test]
fn a_custom_element_with_a_dash_renders() {
	warp::errors::diagnostic::take_runtime_warnings();
	let html = warp::web::markup::to_html(&eval("div{ my-thing{ \"x\" } }"));
	assert_eq!(html, "<div><my-thing>x</my-thing></div>");
	let at_run_time = warp::errors::diagnostic::take_runtime_warnings();
	assert!(at_run_time.is_empty(), "{at_run_time:?}");
	let attributes = warp::web::markup::to_html(&eval("div{ data-x: 1 aria-label: \"l\" \"hi\" }"));
	assert_eq!(attributes, "<div data-x=\"1\" aria-label=\"l\">hi</div>");
}

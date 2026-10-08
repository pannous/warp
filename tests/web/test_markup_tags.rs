// card g-_alg (wiki/mark.md): inside a tag's block `label(for:pwd)` is the tag `label{for:pwd}` and
// `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}`; samples/html.warp, the playground's HTML demo, runs
use crate::common::fails_with;
use crate::eq;
use warp::wasm_emitter::eval;

#[test]
fn a_tag_takes_its_attributes_in_parentheses() {
	eq!(eval("html{ div{ label(for:pwd):\"Password\" } }"), eval("html{ div{ label{for:pwd, \"Password\"} } }"));
	eq!(eval("div{ label(for:pwd a:b) }"), eval("div{ label{for:pwd a:b} }"));
	runs("samples/html.warp");
}

// card g-_bGo: the playground's HTML DSL demo runs; its list items are tags li{…}, children as repeated XML elements
#[test]
fn the_html_dsl_sample_runs() {
	runs("samples/html_dsl.warp");
}

fn runs(sample: &str) {
	let value = eval(sample);
	assert!(value.first_error().is_none(), "{sample}: {}", value.serialize());
}

#[test]
fn a_call_outside_tags_or_of_a_function_stays_a_call() {
	fails_with("label(for:pwd)", "undefined function: label");
	eq!(eval("label(x) := x + 1; div{ label(x:1) }"), eval("div{ 2 }"));
}

// card markup-attribute: a statement `html(lang: "en"){ … }` is the element with its attributes before its children
#[test]
fn an_element_statement_takes_its_attributes_in_parentheses() {
	eq!(eval("html(lang: \"en\"){ p{ \"x\" } }"), eval("html{ lang: \"en\"; p{ \"x\" } }"));
	eq!(eval("div(class: \"box\" id: \"a\") { p{ \"x\" } }"), eval("div{ class: \"box\" id: \"a\" p{ \"x\" } }"));
}

// card markup-attribute: rendering a value as HTML (what the CLI prints) repeats none of the program's warnings
#[test]
fn rendering_markup_warns_nothing_more() {
	let shown = eval("html{ p{ \"x\" } }");
	warp::diagnostic::take_warnings();
	assert_eq!(warp::markup::to_html(&shown), "<html><p>x</p></html>");
	assert_eq!(warp::diagnostic::take_warnings().len(), 0);
}

// card g-_bGo, user decision P176: in a named tag's block a repeated `key: value` is a child, as repeated elements in
// XML/HTML (`ul{ li: "First" li: "Second" }` holds two li), glued or spaced; a plain `{…}` stays a map whose
// repeated key is an error, also inside a tag, and so does a declared type's constructor
#[test]
fn repeated_keys_in_a_tag_block_are_children() {
	eq!(eval("ul{ li: \"First\" li: \"Second\" }"), eval("ul{ li{ \"First\" } li{ \"Second\" } }"));
	eq!(eval("ul { li: \"First\" li: \"Second\" }"), eval("ul{ li{ \"First\" } li{ \"Second\" } }"));
	eq!(eval("div{ class: \"x\" p: \"a\" p: \"b\" }"), eval("div{ class: \"x\" p{ \"a\" } p{ \"b\" } }"));
	fails_with("{ a: 1 a: 2 }", "duplicate key 'a'");
	fails_with("ul{ li: \"a\" data: { b: 1 b: 2 } }", "duplicate key 'b'");
	fails_with("class Point { x: int y: int }; Point{ x: 1 x: 2 }", "duplicate key 'x'");
}


// P188 (stay close to HTML): HTML's own attribute form name="value" inside a tag's block is the attribute name:"value",
// also for the keywords type and class and in parentheses; a name that is no attribute stays an assignment
#[test]
fn a_tag_takes_html_attributes_with_equals() {
	let html_of = |code: &str| warp::markup::to_html(&eval(code));
	assert_eq!(html_of("label{\"Name\" input{type=\"text\" style=\"color: blue\"}}"), "<label>Name<input type=\"text\" style=\"color: blue\"></label>");
	assert_eq!(html_of("p{class=\"card\" id=\"main\" \"x\"}"), "<p class=\"card\" id=\"main\">x</p>");
	assert_eq!(html_of("a{href=\"/about\" data-page=\"about\" \"About\"}"), "<a href=\"/about\" data-page=\"about\">About</a>");
	assert_eq!(html_of("div{ label(for=\"pwd\"):\"Password\" }"), "<div><label for=\"pwd\">Password</label></div>");
	assert_eq!(html_of("p{ shown = \"seen\" }"), "<p>seen</p>");
}

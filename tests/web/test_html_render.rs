//! A markup value renders as HTML (card web-dom, notes/web_framework.md): tags with their attributes, text escaped,
//! children in order, void elements without a closing tag; anything else is no markup
use warp::markup::{is_markup, to_html};
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	let value = eval(code);
	assert!(is_markup(&value), "no markup: {}", value.serialize());
	to_html(&value)
}

#[test]
fn tags_attributes_and_text() {
	assert_eq!(html_of("div{ class:\"box\" h1{ \"Hi\" } p{ \"x < y & z\" } }"), "<div class=\"box\"><h1>Hi</h1><p>x &lt; y &amp; z</p></div>");
	assert_eq!(html_of("ul{ li{ \"a\" } li{ \"b\" } }"), "<ul><li>a</li><li>b</li></ul>");
	assert_eq!(html_of("p{ a{ href:\"/home\" \"Home\" } \" and more\" }"), "<p><a href=\"/home\">Home</a> and more</p>");
}

#[test]
fn void_elements_and_attribute_lists() {
	assert_eq!(html_of("div{ input{ type:email id:email } br{} button{ class:['btn' 'btn-info'] \"Go\" } }"),
		"<div><input type=\"email\" id=\"email\"><br><button class=\"btn btn-info\">Go</button></div>");
}

#[test]
fn a_whole_page() {
	let page = html_of("html{ head{ title: \"My Page\" meta{ charset: \"utf-8\" } } body{ h1: \"Welcome\" } }");
	assert_eq!(page, "<html><head><title>My Page</title><meta charset=\"utf-8\"></head><body><h1>Welcome</h1></body></html>");
}

#[test]
fn other_values_are_no_markup() {
	for code in ["3", "\"div\"", "{name: \"Alice\"}", "person{ name: \"Alice\" }"] {
		assert!(!is_markup(&eval(code)), "{code}");
	}
}

#[test]
fn the_page_gets_the_html_of_markup() {
	let report = warp::web::evaluate("ul{ li{ \"a\" } }", Default::default());
	assert_eq!(report["html"], "<ul><li>a</li></ul>");
	assert!(warp::web::evaluate("3", Default::default())["html"].is_null());
}

// an element's computed child next to an attribute is a child, not part of one sum (card markup-arithmetic)
#[test]
fn a_computed_child_next_to_an_attribute() {
	assert_eq!(html_of("count = 3\np{ class: \"x\" \"c\" + count }"), "<p class=\"x\">c3</p>");
	assert_eq!(html_of("count = 3\ndiv{ p{ style: { fontSize: 12 } \"c\" + count } }"), "<div><p style=\"font-size: 12px\">c3</p></div>");
}

// a for loop among an element's children gives its items as children, as [li{t} for t in todos] does (card markup-ul)
#[test]
fn a_for_loop_inside_markup() {
	assert_eq!(html_of("todos = [\"a\", \"b\"]\nul{ for t in todos { li{ t } } }"), "<ul><li>a</li><li>b</li></ul>");
	assert_eq!(html_of("todos = [\"a\", \"b\"]\ndiv{ p:\"y\" for t in todos { span{ t } } }"), "<div><p>y</p><span>a</span><span>b</span></div>");
	assert_eq!(html_of("items = [\"a\"]\nul { for item in items { li: item } }"), "<ul><li>a</li></ul>");
}


// the loop nested in html{ ul{ … } } over numbers (card markup-index: it was 'undefined variable: i·index')
#[test]
fn a_for_loop_nested_in_html() {
	assert_eq!(html_of("items = [1, 2]; html{ lang: \"en\"; ul{ for i in items { li{ i } } } }"), "<html lang=\"en\"><ul><li>1</li><li>2</li></ul></html>");
}

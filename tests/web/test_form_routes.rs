//! card g_mSEw: a form names the route it sends to as the route is written, `form post "/todos" { … }`, instead of
//! spelling the attributes `form{ method:"post" action:"/todos" … }`
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

const POSTING: &str = "<form method=\"post\" action=\"/todos\"><button>add</button></form>";

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_form_written_as_its_route() {
	assert_eq!(html_of("form post \"/todos\" { button{ \"add\" } }"), POSTING);
	assert_eq!(html_of("form{ method:\"post\" action:\"/todos\" button{ \"add\" } }"), POSTING);
}

#[test]
fn a_form_route_inside_an_element() {
	assert_eq!(html_of("div{ form post \"/todos\" { button{ \"add\" } } }"), format!("<div>{POSTING}</div>"));
	assert_eq!(
		html_of("id = 2\nli{ form post \"/todos/\\(id)/toggle\" { button{ \"☐\" } } \"tea\" }"),
		"<li><form method=\"post\" action=\"/todos/2/toggle\"><button>☐</button></form>tea</li>"
	);
}

// decision function-words-consume: among an element's children a function word takes as many as it has parameters,
// the children after them stay children; form is lib/markup.warp's form(method: symbol, action: text, body)
#[test]
fn a_call_among_children_takes_its_parameters() {
	assert_eq!(html_of("g(a: int, b: int) := a + b\nli{ g 1 2 \"tea\" }"), "<li>3tea</li>");
	assert_eq!(
		html_of("li{ \"tea\" form get \"/t\" { button{ \"x\" } } }"),
		"<li>tea<form method=\"get\" action=\"/t\"><button>x</button></form></li>"
	);
}

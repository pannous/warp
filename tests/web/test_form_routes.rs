//! card g_mSEw: a form names the route it sends to as the route is written, `form post "/todos" { … }`, instead of
//! spelling the attributes `form{ method:"post" action:"/todos" … }`
use warp::markup::to_html;
use warp::wasm_emitter::eval;

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
	assert_eq!(html_of("id = 2\nli{ form post \"/todos/\\(id)/toggle\" { button{ \"☐\" } } \"tea\" }"),
		"<li><form method=\"post\" action=\"/todos/2/toggle\"><button>☐</button></form>tea</li>");
}

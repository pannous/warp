//! Class components (notes/web_framework.md step 4, notes/classes.md): an instance of a class with a render() method
//! is its markup where markup is expected
use warp::html::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

const GREETING: &str = "class Greeting{ name:text; render() := p{ \"Hi \" + name } }\n";

#[test]
fn an_instance_in_markup_renders() {
	assert_eq!(html_of(&format!("{GREETING}div{{ Greeting(\"Ann\") Greeting(\"Bo\") }}")), "<div><p>Hi Ann</p><p>Hi Bo</p></div>");
}

#[test]
fn an_instance_as_the_programs_value_renders() {
	assert_eq!(html_of(&format!("{GREETING}Greeting(\"Ann\")")), "<p>Hi Ann</p>");
}

#[test]
fn other_frameworks_names_of_render() {
	assert_eq!(html_of("class Badge{ n:int; view() := span{ \"#\\(n)\" } }\ndiv{ Badge(3) }"), "<div><span>#3</span></div>");
	assert_eq!(html_of("class Badge{ n:int; build() := span{ \"#\\(n)\" } }\nBadge(4)"), "<span>#4</span>");
}

#[test]
fn an_instance_outside_markup_stays_an_instance() {
	assert_eq!(eval(&format!("{GREETING}g = Greeting(\"Ann\"); g.name")).serialize().trim(), "\"Ann\"");
}

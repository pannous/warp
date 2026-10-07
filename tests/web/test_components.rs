//! Components are functions returning markup (card web-components, notes/web_framework.md step 4): props are its
//! parameters, positional or named, and a block after the call is its last argument, the children
use warp::markup::to_html;
use warp::wasm_emitter::eval;

const CARD: &str = "def Card(title, children) = div{ class:\"card\" h2{ title } children }\n";

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_component_is_a_function_returning_markup() {
	assert_eq!(html_of("def Card(title) = div{ h2{ title } }\ndiv{ Card(\"A\") Card(title:\"B\") }"), "<div><div><h2>A</h2></div><div><h2>B</h2></div></div>");
}

#[test]
fn the_block_after_the_call_is_the_children() {
	let card = "<div class=\"card\"><h2>Hi</h2><p>text</p><em>x</em></div>";
	assert_eq!(html_of(&format!("{CARD}Card(\"Hi\") {{ p:\"text\" em:\"x\" }}")), card);
	assert_eq!(html_of(&format!("{CARD}Card(title:\"Hi\") {{ p:\"text\" em:\"x\" }}")), card);
	assert_eq!(html_of(&format!("{CARD}section{{ Card(\"Hi\") {{ p:\"text\" em:\"x\" }} }}")), format!("<section>{card}</section>"));
}

// each instance keeps its own state (component_state.rs): the page passes the clicked instance as event.instance
// (tour example "components", test_in_browser.py --examples clicks it)
const COUNTER: &str = "def Counter(start) {\n\tcount = start\n\tdiv{ button{ on click { count += 1 } \"+\" } p{ \"n \" + count } }\n}\n";

#[test]
fn each_instance_of_a_component_has_its_own_state() {
	let counter = |instance: usize, count: usize| format!("<div><button data-wasp-instance=\"{instance}\" data-wasp-click=\"1\">+</button><p>n {count}</p></div>");
	assert_eq!(html_of(&format!("{COUNTER}div{{ Counter(1) Counter(5) }}")), format!("<div>{}{}</div>", counter(1, 1), counter(2, 5)));
	assert_eq!(html_of(&format!("{COUNTER}Counter(3)")), counter(1, 3));
}

#[test]
fn a_trailing_block_fills_a_function_parameter() {
	assert_eq!(eval("def apply(x, f) = f(x)\napply(3) { it * 2 }").serialize(), "6");
}

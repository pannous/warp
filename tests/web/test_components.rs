//! Components are functions returning markup (card web-components, notes/web_framework.md step 4): props are its
//! parameters, positional or named, and a block after the call is its last argument, the children
use warp::html::to_html;
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

#[test]
fn a_trailing_block_fills_a_function_parameter() {
	assert_eq!(eval("def apply(x, f) = f(x)\napply(3) { it * 2 }").serialize(), "6");
}

//! card markup-content: an element where HTML does not let it be warns where it is written, as the browser would move
//! or drop it: a block inside p, anything but li in a list, a table cell outside a row, html inside markup. An element
//! with no element around it (a component's own markup) is not judged
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
fn misplaced_elements_warn() {
	warns("p{ div{ \"x\" } }", "div inside p: the browser ends the p before it");
	warns("ul{ p{ \"x\" } }", "p inside ul: a list holds li items");
	warns("div{ td{ \"x\" } }", "td inside div: it belongs in a tr");
	warns("div{ html{ \"x\" } }", "html inside div: html is the page itself");
}

#[test]
fn right_nesting_stays_quiet() {
	for code in [
		"p{ b{ \"x\" } \"y\" }",
		"ul{ li{ \"x\" } }",
		"table{ tr{ td{ \"x\" } } }",
		"table{ tbody{ tr{ th{ \"x\" } } } }",
		"cell(x) := td{ x }\ncell(1)",
		"html{ body{ p{ \"x\" } } }",
	] {
		let mistakes: Vec<String> =
			warnings_of(code).into_iter().filter(|warning| warning.contains(" inside ")).collect();
		assert_eq!(mistakes, Vec::<String>::new(), "{code}");
	}
}

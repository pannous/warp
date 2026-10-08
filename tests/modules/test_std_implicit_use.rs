// card std-implicit (notes/stdlib.md §8 "One implicit-use mechanism"): modules.rs brings the standard modules a program
// needs without `use` from one table (a file URL → file, a page → markup, routes → router, a route's regular expression
// → regex); the lowering passes write no `use` statements of their own
use warp::node::Node;

const ROUTED: &str = r#"route "/users/:id(\\d+)" { p{ "user " + id } }
route "*" { p{ "not found" } }"#;

fn has_use_statement(program: &Node) -> bool {
	program.serialize().contains("use ")
}

#[test]
fn routes_and_pages_lower_without_use_statements() {
	let routed = warp::routes::lower(warp::warp_parser::parse(ROUTED));
	assert!(!has_use_statement(&routed), "{}", routed.serialize());
	let page = warp::pipeline::for_a_page(|| warp::page_html::use_markup(warp::warp_parser::parse("p{ \"hi\" }")));
	assert!(!has_use_statement(&page), "{}", page.serialize());
}

#[cfg(feature = "native")] // the page's path is a native host's
#[test]
fn routes_bring_router_and_regex_implicitly() {
	use warp::{host::with_page_path, markup::to_html, wasm_emitter::eval};
	assert_eq!(with_page_path("/users/7", || to_html(&eval(ROUTED))), "<p>user 7</p>");
	assert_eq!(with_page_path("/users/bo", || to_html(&eval(ROUTED))), "<p>not found</p>");
}

#[test]
fn a_page_brings_markup_implicitly() {
	let module = warp::pipeline::for_a_page(|| warp::pipeline::compile("p{ \"hi\" }")).expect("compiled");
	assert!(!module.bytes.is_empty());
}

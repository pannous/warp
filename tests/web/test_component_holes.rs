//! Card web-fine-components (notes/web_framework.md step 3): a page with components has holes inside them too. Each
//! instance's elements holding a computed part are `page·hole·<path>`, the path running through the instance's root
//! element; `page·holes` renders once, and each hole is its element in that rendering, the instance's own state shown.
//! Only a program rendering itself (the playground's) has them
use warp::run::wasm_reader::{read_exports_after_main, Imports};

const FRUIT: &str = r#"def Counter(label, start) {
	count = start
	div{ button{ on click { count += 1 } "+" } p{ label + ": " + count } }
}
div{ h1{ "Fruit" } Counter("Apples", 1) Counter("Pears", 5) }"#;

#[test]
fn each_instance_binds_its_own_holes() {
	let lowered = warp::pipeline::rendering_itself(|| warp::pipeline::lower(FRUIT)).expect("lowers").serialize();
	for hole in ["(page·holes):=", "(page·hole·1·1):=", "(page·hole·2·1):="] {
		assert!(lowered.contains(hole), "{hole} in {lowered}");
	}
	assert!(!lowered.contains("(page·hole·0)"), "the heading is fixed: {lowered}");
}

#[test]
fn a_hole_is_its_element_of_the_rendering() {
	let module = warp::pipeline::rendering_itself(|| warp::pipeline::compile(FRUIT)).expect("compiles");
	let imports = Imports { host: true, wasi: true, ffi: false };
	let read = read_exports_after_main(&module.bytes, imports, &["page·holes", "page·hole·1·1", "page·hole·2·1"])
		.expect("the holes read");
	assert_eq!(warp::web::markup::to_html(&read[1]), "<p>Apples: 1</p>");
	assert_eq!(warp::web::markup::to_html(&read[2]), "<p>Pears: 5</p>");
}

#[test]
fn an_instances_lone_text_is_a_text_hole() {
	let code = r#"def Tally(start) {
	n = start
	section{ button{ on click { n += 1 } "add" } "clicked " + n + " times" }
}
div{ Tally(2) Tally(7) }"#;
	let module = warp::pipeline::rendering_itself(|| warp::pipeline::compile(code)).expect("compiles");
	let imports = Imports { host: true, wasi: true, ffi: false };
	let read = read_exports_after_main(&module.bytes, imports, &["page·holes", "page·hole·1·text·1"])
		.expect("the text hole reads");
	assert_eq!(read[1].serialize(), "\"clicked 7 times\"");
}

// the readers the holes call: html_element_at walks the element indices, as the page's `children` count them,
// through groups; html_text_at is the text after that many child elements
#[test]
fn the_hole_readers_find_elements_and_texts_by_path() {
	let page = r#"use markup
shown = div{ h1{ "T" } [ p{ "a" } section{ b{ "x" } "after b" i{ "y" } } ] }"#;
	assert_eq!(
		warp::wasm_emitter::eval(&format!("{page}\nto_html(html_element_at(shown, [2, 0]))")),
		warp::node::Node::Text("<b>x</b>".into())
	);
	assert_eq!(
		warp::wasm_emitter::eval(&format!("{page}\nhtml_text_at(shown, [2], 1)")),
		warp::node::Node::Text("after b".into())
	);
}

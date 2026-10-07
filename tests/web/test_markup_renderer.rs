// card web-ssr: lib/markup.wasp's to_html is the one renderer: the CLI and the playground render a markup value with
// it too (src/markup.rs), as a page renders itself (page·html)
use warp::markup::to_html;
use warp::wasm_emitter::eval;

#[test]
fn a_style_sheet_inside_a_scoped_element_styles_only_its_elements() {
	let card = eval("div{ data-wasp-scope:\"Card\" style{ \"h2, p\": { color: navy } } h2{ \"A\" } }");
	assert_eq!(to_html(&card), "<div data-wasp-scope=\"Card\"><style>[data-wasp-scope=\"Card\"] h2, [data-wasp-scope=\"Card\"] p { color: navy }</style><h2>A</h2></div>");
}

#[test]
fn the_renderer_keeps_its_locals_apart_from_the_programs_variables() {
	let program = "use markup\nitems = 3\nout = 1\nto_html(ul{ li{ \"a\" key: items } })";
	assert_eq!(eval(program), warp::node::Node::Text("<ul><li data-wasp-key=\"3\">a</li></ul>".into()));
}

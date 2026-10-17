// card web-ssr: lib/markup.warp's to_html is the one renderer: the CLI and the playground render a markup value with
// it too (src/web/markup.rs), as a page renders itself (page·html)
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

#[test]
fn a_style_sheet_inside_a_scoped_element_styles_only_its_elements() {
	let card = eval("div{ data-warp-scope:\"Card\" style{ \"h2, p\": { color: navy } } h2{ \"A\" } }");
	assert_eq!(to_html(&card), "<div data-warp-scope=\"Card\"><style>[data-warp-scope=\"Card\"] h2, [data-warp-scope=\"Card\"] p { color: navy }</style><h2>A</h2></div>");
}

#[test]
fn the_renderer_keeps_its_locals_apart_from_the_programs_variables() {
	let program = "use markup\nitems = 3\nout = 1\nto_html(ul{ li{ \"a\" key: items } })";
	assert_eq!(eval(program), warp::node::Node::Text("<ul><li data-warp-key=\"3\">a</li></ul>".into()));
}

// an element made by a loop among an element's children keeps its attributes beside a computed child, a condition
// in parentheses or without (card parenthesized-attribute)
#[test]
fn an_element_of_a_loop_keeps_its_attributes_beside_a_condition() {
	let cell = "<td class=\"log\" title=\"t\">y</td>";
	assert_eq!(to_html(&eval("table{ for j in [1] { td{ class:\"log\" title:(if yes then \"t\" else \"\") (if yes then \"y\" else \"\") } } }")), format!("<table>{cell}</table>"));
	assert_eq!(
		to_html(&eval("table{ [td{ class:\"log\" title:\"t\" (if yes then \"y\" else \"\") } for j in [1]] }")),
		format!("<table>{cell}</table>")
	);
	assert_eq!(to_html(&eval("td{ class:\"log\" title:\"t\" if yes then \"y\" else \"\" }")), cell);
}

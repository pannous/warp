//! CSS written as CSS in a style block (card web-styles, src/lowering/style_rules.rs): selectors without quotes and
//! lengths with units, one rule per line
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn selectors_need_no_quotes() {
	let sheet = "div{\n\tstyle{\n\t\t.card { color: \"navy\" }\n\t\tul > li { margin: 0 }\n\t\th1, h2 { color: \"red\" }\n\t\ta:hover { color: \"red\" }\n\t\tp.note { margin: 0 }\n\t\tul li { margin: 0 }\n\t\t.a.b { margin: 0 }\n\t\t#main { margin: 0 }\n\t}\n}";
	let rules = ".card { color: navy } ul > li { margin: 0px } h1, h2 { color: red } a:hover { color: red } p.note { margin: 0px } ul li { margin: 0px } .a.b { margin: 0px } #main { margin: 0px }";
	assert_eq!(html_of(sheet), format!("<div><style>{rules}</style></div>"));
}

#[test]
fn lengths_keep_their_units() {
	assert_eq!(
		html_of("div{ style: { padding: 8px 4px; margin: 0 auto; width: 50%; left: -2px; fontSize: 1.5em } \"x\" }"),
		"<div style=\"padding: 8px 4px; margin: 0 auto; width: 50%; left: -2px; font-size: 1.5em\">x</div>"
	);
	assert_eq!(
		html_of("div{ style{\n\t.card { border: 1px solid \"red\" }\n} }"),
		"<div><style>.card { border: 1px solid red }</style></div>"
	);
	assert_eq!(html_of("p{ style: { flex: 1 1 } \"x\" }"), "<p style=\"flex: 1 1\">x</p>");
}

// a blank before `.x` or `#x` in a selector is the descendant combinator, and a rule after another on the same line
// is a rule of its own, not an index into it (card web-styles-parser)
#[test]
fn descendant_selectors_and_rules_on_one_line() {
	assert_eq!(
		html_of("div{ style{\n\t#main .x { margin: 0 }\n\tul .item { margin: 0 }\n\t.a .b #c { margin: 0 }\n} }"),
		"<div><style>#main .x { margin: 0px } ul .item { margin: 0px } .a .b #c { margin: 0px }</style></div>"
	);
	assert_eq!(
		html_of("div{ style{ .a { margin: 0 } #main { margin: 0 } } }"),
		"<div><style>.a { margin: 0px } #main { margin: 0px }</style></div>"
	);
}

// a class or id selector with a lone value is its color (card style-selectors); one declaration in braces stays a rule
#[test]
fn a_selector_with_a_value_is_its_color() {
	assert_eq!(html_of("div{ style{ #done = \"red\" } }"), "<div><style>#done { color: red }</style></div>");
	assert_eq!(
		html_of("div{ style{ .done: \"red\"; .open = green } }"),
		"<div><style>.done { color: red } .open { color: green }</style></div>"
	);
	assert_eq!(
		html_of("div{ style{ .done{color:red} #main{margin:0} } }"),
		"<div><style>.done { color: red } #main { margin: 0px }</style></div>"
	);
}

// a hex color is CSS's, not the count or index operator: `solid #8884`, `color: #fff` (card css-block); an id
// selector of hex letters stays a selector
#[test]
fn hex_colors_in_a_style_sheet() {
	assert_eq!(html_of("div{ style{\n\ttd { border: 1px solid #8884 }\n\t#main { color: #fff }\n\tp { color: #0a0 }\n\tb { color: #1e1e1e }\n\ti { color: #00ff00 }\n\t#fed { color: #abc }\n} }"),
		"<div><style>td { border: 1px solid #8884 } #main { color: #fff } p { color: #0a0 } b { color: #1e1e1e } i { color: #00ff00 } #fed { color: #abc }</style></div>");
}

// `:root` and a rule named after an element (`html { … }`, `body { … }`) are rules of the sheet (card markup-style-block)
#[test]
fn root_and_element_rules_in_a_style_sheet() {
	assert_eq!(
		html_of("div{ style{\n\t:root { color-scheme: light dark }\n\thtml { margin: 0 }\n\tbody { margin: 0 }\n} }"),
		"<div><style>:root { color-scheme: light dark } html { margin: 0px } body { margin: 0px }</style></div>"
	);
	assert_eq!(
		html_of("div{ style{ h1, :hover { margin: 0 } h2, .x { margin: 0 } ul li, .y { margin: 0 } } }"),
		"<div><style>h1, :hover { margin: 0px } h2, .x { margin: 0px } ul li, .y { margin: 0px }</style></div>"
	);
}

// a percentage followed by the next declaration on its line, and several rules with comma selectors on one line
// (card web-styles-limits)
#[test]
fn percentages_and_comma_rules_on_one_line() {
	assert_eq!(
		html_of("div{ style{ .a { width: 50% height: 10px } } }"),
		"<div><style>.a { width: 50%; height: 10px }</style></div>"
	);
	assert_eq!(
		html_of("div{ style{ h1, h2 { margin: 0 } h3, h4 { margin: 1px } } }"),
		"<div><style>h1, h2 { margin: 0px } h3, h4 { margin: 1px }</style></div>"
	);
}

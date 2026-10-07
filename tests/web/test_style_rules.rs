//! CSS written as CSS in a style block (card web-styles, src/lowering/style_rules.rs): selectors without quotes and
//! lengths with units, one rule per line
use warp::html::to_html;
use warp::wasm_emitter::eval;

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
	assert_eq!(html_of("div{ style: { padding: 8px 4px; margin: 0 auto; width: 50%; left: -2px; fontSize: 1.5em } \"x\" }"), "<div style=\"padding: 8px 4px; margin: 0 auto; width: 50%; left: -2px; font-size: 1.5em\">x</div>");
	assert_eq!(html_of("div{ style{\n\t.card { border: 1px solid \"red\" }\n} }"), "<div><style>.card { border: 1px solid red }</style></div>");
	assert_eq!(html_of("p{ style: { flex: 1 1 } \"x\" }"), "<p style=\"flex: 1 1\">x</p>");
}

// a blank before `.x` or `#x` in a selector is the descendant combinator, and a rule after another on the same line
// is a rule of its own, not an index into it (card web-styles-parser)
#[test]
fn descendant_selectors_and_rules_on_one_line() {
	assert_eq!(html_of("div{ style{\n\t#main .x { margin: 0 }\n\tul .item { margin: 0 }\n\t.a .b #c { margin: 0 }\n} }"),
		"<div><style>#main .x { margin: 0px } ul .item { margin: 0px } .a .b #c { margin: 0px }</style></div>");
	assert_eq!(html_of("div{ style{ .a { margin: 0 } #main { margin: 0 } } }"), "<div><style>.a { margin: 0px } #main { margin: 0px }</style></div>");
}


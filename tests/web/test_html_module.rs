// card web-ssr: std/markup.wasp's to_html, the renderer a page runs itself (page·html), writes markup as src/html.rs
// does: one renderer for a built site's index.html and its updates in the browser
use warp::node::Node;

/// Markup with values known only at run time (`n`, `done`), so the program renders it, not the compiler
const MARKUP: [&str; 11] = [
	"div{ class:\"box\" h1{ \"Hi\" } p{ \"n is \" + n } }",
	"ul{ li{ \"first\" key: n } li{ \"second\" } }",
	"input{ type:\"checkbox\" checked: done }",
	"input{ type:\"checkbox\" checked: not done }",
	"p{ \"a < b & \\\"c\\\" \" + n }",
	"div{ class:[\"btn\" \"btn-info\"] br{} span{ n } }",
	"button{ \"Add\" data-wasp-click: n }",
	"\"just text \" + n",
	"[p{ n } p{ n + 1 }]",
	"p{ style: { fontSize: 12 opacity: 0.5 } \"x\" }",
	"style{ \".card\": { padding: 8 } }",
];

#[test]
fn to_html_writes_markup_as_html_rs_does() {
	for markup in MARKUP {
		let program = format!("n = 7\ndone = true\nshown = {markup}\n");
		let expected = warp::html::to_html(&warp::wasm_emitter::eval(&format!("{program}shown")));
		let rendered = warp::wasm_emitter::eval(&format!("use markup\n{program}to_html(shown)"));
		assert_eq!(rendered, Node::Text(expected), "{markup}");
	}
}

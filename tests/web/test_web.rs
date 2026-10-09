// Web/Browser tests
// Migrated from tests_*.rs files

use warp::util::fetch;
use warp::wasm_emitter::eval;
use warp::type_kinds::NodeKind;
use crate::{is, eq};

#[test]
fn test_html_warp() {
	eval("html{bold{Hello}}"); // => <html><body><bold>Hello</bold></body></html> via appendChild bold to body
	eval("html: h1: 'Hello, World!'"); // => <html><h1>Hello, World!</h1></html>
	                                //	eval("html{bold($myid style=red){Hello}}"); // => <bold id=myid style=red>Hello</bold>
}

// a script element holds JavaScript, as written (card vacuous-tests; `js{…}` as its alias: card js-element)
#[test]
fn test_js() {
	is!("use markup; to_html(script{\"alert('Hello')\"})", "<script>alert('Hello')</script>");
	calls_the_page("use js document; ctx = document.getElementById(\"canvas\").getContext(\"2d\")", &["getElementById", "getContext"]);
}

/// A page program reaches the DOM through `use js document`: it compiles to the host's foreign calls naming each member;
/// running it needs a page's document, which neither the native runner nor the browser suite's Worker has: tests/web/test_dom_pages.rs runs such programs on a page
fn calls_the_page(code: &str, members: &[&str]) {
	let bytes = warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}")).bytes;
	for member in members {
		assert!(bytes.windows(member.len()).any(|window| window == member.as_bytes()), "{code} calls no {member}");
	}
}

// markup is HTML, and a page's element takes HTML as its content
#[test]
fn test_inner_html() {
	is!("use markup; to_html(html{b{\"test\"}})", "<html><b>test</b></html>");
	is!("use markup; to_html(html{script{\"alert('ok')\"}})", "<html><script>alert('ok')</script></html>");
	calls_the_page("use js document; results = document.getElementById(\"results\"); results.innerHTML = \"<i>ok</i>\"", &["getElementById", "innerHTML"]);
}

#[test]
fn test_fetch() {
	is!("x=fetch https://pannous.com/files/test;i=7;x", "test 2 5 3 7\n");
	let res = fetch("https://pannous.com/files/test");
	eq!(res, "test 2 5 3 7");
	is!("fetch https://pannous.com/files/test", "test 2 5 3 7\n");
	is!("x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
	is!("string x=fetch https://pannous.com/files/test;y=7;x", "test 2 5 3 7\n");
	is!("string x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
}

// a red rectangle on a canvas: drawn by `use draw` (the playground's canvas, natively a PNG), or on a page's canvas
#[test]
fn test_canvas() {
	let corners = "[canvas_pixels[10 * 200 + 10], canvas_pixels[109 * 200 + 159], canvas_pixels[110 * 200 + 160], canvas_pixels[0]]";
	is!(&format!("use draw\ncanvas(200, 150)\nrect(10, 10, 150, 100, red)\n{corners}.map(p => p == red)"), warp::ints(vec![1, 1, 0, 0]));
	calls_the_page("use js document\nctx = document.getElementById(\"canvas\").getContext(\"2d\")\nctx.fillStyle = \"red\"\nctx.fillRect(10, 10, 150, 100)", &["fillStyle", "fillRect"]);
}

// an element of the page by its id is a WebIDL Element, its members checked at compile time
#[test]
fn test_dom() {
	calls_the_page("use js document; document.getElementById(\"canvas\").id", &["getElementById"]);
	crate::common::fails_with("use js document; document.getElementById(\"canvas\").innerHtml", "did you mean innerHTML");
}

#[test]
#[ignore = "WEBAPP feature required"]
fn test_dom_property() {
	// #[cfg(not(feature = "WEBAPP"))]{
	//     return;
	// }
	let mut result = eval("getExternRefPropertyValue($canvas,'width')"); // ok!!
	eq!(result.value(), &300); // only works because String "300" gets converted to BigInt 300
							//	result = eval("width='width';$canvas.width");
	result = eval("$canvas.width");
	eq!(result.value(), &300);
	//	return;
	result = eval("$canvas.style");
	eq!(result.kind(), NodeKind::Text);
	//	eq!(result.kind, stringp);
	// if (result.value().string);
	// is!(*result.value().string, "dfsa");
	//	getExternRefPropertyValue OK  [object HTMLCanvasElement] style [object CSSStyleDeclaration]
	// ⚠️ But can't forward result as smarti or stringref:  SyntaxError: Failed to parse String to BigInt
	// todo : how to communicate new string as RETURN type of arbitrary function from js to warp?
	// call Webview.getString(); ?

	//	embedder.trace('canvas = document.getElementById("canvas");');
	//	print(nod);
}

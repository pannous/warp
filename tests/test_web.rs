// Web/Browser tests
// Migrated from tests_*.rs files

use wasp::analyzer::analyze;
use wasp::extensions::print;
use wasp::util::fetch;
use wasp::wasm_gc_emitter::eval;
use wasp::wasp_parser::parse;
use wasp::{eq, is, put, skip};

#[test]
fn test_html_wasp() {
	eval("html{bold{Hello}}"); // => <html><body><bold>Hello</bold></body></html> via appendChild bold to body
	eval("html: h1: 'Hello, World!'"); // => <html><h1>Hello, World!</h1></html>
	                                //	eval("html{bold($myid style=red){Hello}}"); // => <bold id=myid style=red>Hello</bold>
}

#[test]
#[ignore]
fn test_js() {
	// todo remove (local $getContext i32)  !
	eval("$canvas.getContext('2d')"); // => invokeReference(canvas, getContext, '2d');
	skip!(

		eval("js{alert('Hello')}"); // => <script>alert('Hello')</script>
		eval("script{alert('Hello')}"); // => <script>alert('Hello')</script>
	);
}

#[test]
fn test_inner_html() {
	#[cfg(not(any(feature = "WEBAPP", feature = "MY_WASM")))]
	{
		return;
	}
	// let html = parse("<html><bold>test</bold></html>");
	// eq!(*html.value(), "<bold>test</bold>");
	// let serialized = html.serialize();
	// eq!(serialized, "<html><bold>test</bold></html>");
	//	eval("<html><script>alert('ok')");
	//	eval("<html><script>alert('ok')</script></html>");
	#[cfg(feature = "WEBAPP")]
	{
		// todo browser "too"
		// skip!(

		eval("<html><bold id=b ok=123>test</bold></html>");
		is!("$b.ok", 123); // TODO emitAttributeSetter
		eval("<script>console.log('ok!')</script>");
		eval("<script>alert('alert ok!')</script>"); // // pop up window NOT supported by WebView, so we use print instead
		                                       // );
	}

	//	eval("$b.innerHTML='<i>ok</i>'");
	//	eval("<html><bold id='anchor'>…</bold></html>");
	//	eval("$anchor.innerHTML='<i>ok</i>'");
	//
	////	eval("x=<html><bold>test</bold></html>;$results.innerHTML=x");
	//	eval("$results.innerHTML='<bold>test</bold>'");
}

#[test]
fn test_html() {
	//	testHtmlWasp();
	//	testJS();
	test_inner_html();
}

#[test]
#[ignore]
fn test_fetch() {
	// todo: use host fetch if available
	let res = fetch("https://pannous.com/files/test");
	if res.contains("not available") {
		print("fetch not available. set CURL=1 in CMakelists.txt or use host function");
		return;
	}
	eq!(res, "test 2 5 3 7");
	is!("fetch https://pannous.com/files/test", "test 2 5 3 7\n");
	is!("x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
	skip!(

		assert!_emit("string x=fetch https://pannous.com/files/test;y=7;x", "test 2 5 3 7\n");
		assert!_emit("string x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
	);
}

#[test]
#[ignore]
fn test_canvas() {
	let _result = analyze(parse("$canvas"));
	// TODO: Externref type not yet implemented in NodeTag
	// eq!(result.kind(), NodeKind::Externref);
	let nod = eval("    ctx = $canvas.getContext('2d');\n    ctx.fillStyle = 'red';\n    ctx.fillRect(10, 10, 150, 100);");
	put!(nod);
}

#[test]
#[ignore]
fn test_dom() {
	print("test_dom");
	// preRegisterFunctions();
	let mut _result = analyze(parse("getElementById('canvas')"));
	// eq!(result.kind(), AstKind::Call);
	_result = eval("getElementById('canvas');");
	//	print(typeName(result.kind));
	//	eq!(result.kind(), strings); // why?
	//	eq!(result.kind(), longs); // todo: can't use smart pointers for elusive externref
	//	eq!(result.kind(), bools); // todo: can't use smart pointers for elusive externref
	// print(typeName(30));
	// print(typeName(9));
	//	eq!(result.kind(), 30);//
	//	eq!(result.kind(),9);//
	//	eq!(result.kind(),  externref); // todo: can't use smart pointers for elusive externref
	//	result = eval("document.getElementById('canvas');");
	//	result = analyze(parse("$canvas"));
	//	eq!(result.kind(),  externref);
}

#[test]
fn test_dom_property() {
	#[cfg(not(feature = "WEBAPP"))]
	{
		return;
	}
	#[cfg(feature = "WEBAPP")]
	{
		let mut result = eval("getExternRefPropertyValue($canvas,'width')"); // ok!!
		eq!(result.value(), &300); // only works because String "300" gets converted to BigInt 300
							 //	result = eval("width='width';$canvas.width");
		result = eval("$canvas.width");
		eq!(result.value(), &300);
		//	return;
		result = eval("$canvas.style");
	}
	// eq!(result.kind(), strings);
	//	eq!(result.kind(), stringp);
	// if (result.value()));
	// is!(*result.value()), "dfsa");
	//	getExternRefPropertyValue OK  [object HTMLCanvasElement] style [object CSSStyleDeclaration]
	// ⚠️ But can't forward result as smarti or stringref:  SyntaxError: Failed to parse String to BigInt
	// todo : how to communicate new string as RETURN type of arbitrary function from js to wasp?
	// call Webview.getString(); ?

	//	embedder.trace('canvas = document.getElementById("canvas");');
	//	print(nod);
}

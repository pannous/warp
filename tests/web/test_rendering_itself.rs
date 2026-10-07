// card playground-render: a playground program holding markup renders itself: compiled to export page·html and
// page·render (std/markup.wasp's to_html inside its own module), the worker no longer compiles a renderer program for
// each value and each fine hole
use warp::page_html::{PAGE_HTML, PAGE_RENDER};

fn exports_of(code: &str) -> Vec<String> {
	let module = warp::pipeline::rendering_itself(|| warp::pipeline::compile(code)).expect("compiled");
	let exported = wasmparser::Parser::new(0).parse_all(&module.bytes).filter_map(Result::ok).filter_map(|payload| match payload {
		wasmparser::Payload::ExportSection(exports) => Some(exports.into_iter().flatten().map(|export| export.name.to_string()).collect::<Vec<_>>()),
		_ => None,
	});
	exported.flatten().collect()
}

#[test]
fn a_program_with_markup_exports_its_renderer() {
	let exports = exports_of("div{ p{ \"hi\" } }");
	assert!(exports.iter().any(|name| name == PAGE_HTML) && exports.iter().any(|name| name == PAGE_RENDER), "{exports:?}");
}

#[test]
fn only_a_program_with_markup_renders_itself() {
	assert!(warp::web::renders_itself("count = 0\ndiv{ button{ on click { count += 1 } \"Add\" } p{ count } }"));
	assert!(!warp::web::renders_itself("x = 3\nx * 2"));
}

#[cfg(feature = "native")]
#[test]
fn the_renderer_inside_the_module_gives_the_html() {
	let module = warp::pipeline::rendering_itself(|| warp::pipeline::compile("div{ p{ \"hi\" } }")).expect("compiled");
	let imports = warp::wasm_reader::Imports { host: true, wasi: true, ffi: false };
	let html = warp::wasm_reader::read_export_after_main(&module.bytes, imports, PAGE_HTML).expect("rendered");
	assert_eq!(html.serialize().trim(), "\"<div><p>hi</p></div>\"");
}

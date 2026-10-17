// card takes-three: a built site ships the WebAssembly modules its program imports by path, beside the page as
// modules/<file>, and its imports name them `page:modules/<file>` instead of a path of the building machine
use crate::common::scratch_directory;

#[test]
fn a_site_ships_the_modules_its_program_imports() {
	let directory = scratch_directory("shout-site");
	let program = "import tests/fixtures/wasm/shout\np{ shout(\"ab\") }";
	let site = warp::web::site::build(program, "shout", &directory).expect("the site is built");
	assert!(site.files.contains(&"modules/shout.wasm".to_string()), "{:?}", site.files);
	assert!(std::fs::read_to_string(directory.join("index.html")).unwrap().contains("AB!"));
	let program = std::fs::read(directory.join("app.wasm")).unwrap();
	let imports = warp::host::host_parts::imports_of(&program).unwrap();
	assert!(imports.iter().any(|(module, name)| module == "page:modules/shout.wasm" && name == "shout"), "{imports:?}");
}

// card imports-wasm: `warp compile --page` writes a module for a page of its own (web/uniscript) the same way, so the
// committed module names no path of the building machine
#[test]
fn a_page_module_ships_the_modules_it_imports_beside_it() {
	let directory = scratch_directory("shout-page");
	let program = warp::pipeline::compile("import tests/fixtures/wasm/shout\nshout(\"ab\")").expect("it compiles");
	let module = directory.join("shout_page.wasm");
	let written = warp::web::site_modules::write_page_module(&program.bytes, &module).expect("the module is written");
	assert_eq!(written, vec!["modules/shout.wasm".to_string()]);
	assert!(directory.join("modules/shout.wasm").exists());
	let imports = warp::host::host_parts::imports_of(&std::fs::read(&module).unwrap()).unwrap();
	assert!(imports.iter().any(|(module, name)| module == "page:modules/shout.wasm" && name == "shout"), "{imports:?}");
	assert!(!imports.iter().any(|(module, _)| module.starts_with('/')), "{imports:?}");
}

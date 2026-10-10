// card local-lib: lib/<std>.warp of another warp checkout (the text the binary embeds, found in a folder other than the
// build's) is the standard module, its nested `use math` too; a file of different text shadows it, with a warning
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn checkout_with(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
	let folder = crate::common::scratch_directory(name);
	std::fs::create_dir_all(folder.join("lib")).expect("scratch lib folder");
	for (module, text) in files {
		std::fs::write(folder.join("lib").join(format!("{module}.warp")), text).expect("the lib file");
	}
	folder
}

fn embedded(module: &str) -> &'static str {
	warp::modules::std_module_source(module).expect("a standard module")
}

#[test]
fn another_checkout_lib_is_the_standard_module() {
	let folder = checkout_with("checkout_std_lib_copy", &[("draw", embedded("draw")), ("math", embedded("math"))]);
	take_warnings();
	let result = warp::modules::with_program_file(&folder.join("lib").join("app.warp"), || eval("use draw\nhsv(120, 1, 1)"));
	assert_eq!(result.serialize().trim(), "4278255360");
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("shadows")));
}

#[test]
fn a_changed_lib_file_shadows_the_standard_module_loudly() {
	let folder = checkout_with("checkout_std_lib_changed", &[("draw", "answer := 42")]);
	take_warnings();
	let result = warp::modules::with_program_file(&folder.join("lib").join("app.warp"), || eval("use draw\nanswer"));
	assert_eq!(result.serialize().trim(), "42");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("shadows the standard module draw")));
}

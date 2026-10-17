// card sample-local: a file beside the program named like a registered package shadows the package, with a warning
// naming both (samples/uniscript.warp made `use uniscript >= 1.0.0` load the sample: "declares no version")
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn run_beside(folder_name: &str, program: &str) -> (String, Vec<String>) {
	let folder = crate::common::scratch_directory(folder_name);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	std::fs::write(folder.join("uniscript.warp"), "answer := 7").expect("the shadowing file");
	take_warnings();
	let result = warp::modules::with_program_file(&folder.join("app.warp"), || eval(program));
	let warnings = take_warnings().into_iter().map(|warning| warning.message).collect();
	(result.serialize(), warnings)
}

fn names_both(warnings: &[String]) -> bool {
	warnings.iter().any(|warning| {
		warning.contains("uniscript.warp shadows the package uniscript") && warning.contains("github.com")
	})
}

#[test]
fn a_file_named_like_a_package_shadows_it_loudly() {
	let (result, warnings) = run_beside("package_shadow_plain", "use uniscript\nanswer");
	assert_eq!(result.trim(), "7");
	assert!(names_both(&warnings), "{warnings:?}");
}

#[test]
fn a_required_version_names_the_shadowing_file_too() {
	let (result, warnings) = run_beside("package_shadow_version", "use uniscript >= 1.0.0\nanswer");
	assert!(result.contains("declares no version"), "{result}");
	assert!(names_both(&warnings), "{warnings:?}");
}

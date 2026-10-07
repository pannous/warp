// card cli-std: a program whose file is named like a standard module (hash.wasp saying `use hash`) uses the standard
// module, never its own file
use warp::wasm_emitter::eval;

#[test]
fn a_program_named_like_a_std_module_uses_the_std_module() {
	let folder = crate::common::scratch_directory("std_named_program");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let code = "use hash; crc32(\"abc\")";
	let program = folder.join("hash.wasp");
	std::fs::write(&program, code).expect("the program file");
	let result = warp::modules::with_program_file(&program, || eval(code));
	assert_eq!(result.serialize().trim(), "891568578");
}

// a file of the program's folder named like a standard module comes first (a local module wins); a word of the standard
// module it lacks says which file took its place
#[test]
fn a_file_shadowing_a_std_module_is_named() {
	let folder = crate::common::scratch_directory("std_shadowing_file");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	std::fs::write(folder.join("hash.wasp"), "greeting = \"hi\"").expect("the shadowing file");
	let code = "use hash; crc32(\"abc\")";
	let result = warp::modules::with_program_file(&folder.join("app.wasp"), || eval(code));
	let message = result.serialize();
	assert!(message.contains("finds the file") && message.contains("hash.wasp"), "{message}");
}

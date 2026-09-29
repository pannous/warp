use std::path::PathBuf;
use std::process::Command;
use warp::wasm_emitter::run_raw_struct;
use warp::wasm_reader::read_bytes;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");
const STRUCT_SOURCE: &str = "class Person{name:String age:i64}; Person{name:'Alice' age:30}";

fn compile_source(file_name: &str, source: &str) -> Vec<u8> {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(file_name);
	let binary_path = source_path.with_extension("wasm");
	std::fs::write(&source_path, source).unwrap();
	let _ = std::fs::remove_file(&binary_path);

	let output = Command::new(env!("CARGO_BIN_EXE_warp")).arg("compile").arg(&source_path).output().unwrap();

	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert!(!String::from_utf8_lossy(&output.stdout).contains("» "), "compile must not print a result");
	std::fs::read(&binary_path).expect("compile wrote no .wasm file")
}

#[test]
fn compile_command_writes_wasm_without_running() {
	let bytes = compile_source("compile_only_answer.warp", "6*7");
	assert_eq!(read_bytes(&bytes).unwrap().serialize(), "42");
}

#[test]
fn compile_command_emits_the_same_struct_module_as_eval() {
	let bytes = compile_source("compile_only_struct.warp", STRUCT_SOURCE);
	let instance = run_raw_struct(&bytes).expect("compile must write the raw struct module that eval runs");
	assert!(format!("{instance:?}").contains("Person{"));
}

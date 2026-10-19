use std::path::PathBuf;
use warp::run::wasm_reader::read_bytes;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

fn compile_source(file_name: &str, source: &str) -> Vec<u8> {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(file_name);
	let binary_path = source_path.with_extension("wasm");
	std::fs::write(&source_path, source).unwrap();
	let _ = std::fs::remove_file(&binary_path);

	let output = crate::common::warp_command().args(["compile", "--wasm"]).arg(&source_path).output().unwrap();

	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert!(!String::from_utf8_lossy(&output.stdout).contains("» "), "compile must not print a result");
	std::fs::read(&binary_path).expect("compile wrote no .wasm file")
}

#[test]
fn compile_command_writes_wasm_without_running() {
	let bytes = compile_source("compile_only_answer.warp", "6*7");
	assert_eq!(read_bytes(&bytes).unwrap().serialize(), "42");
}

use std::fs;
use std::path::PathBuf;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// `warp compile --aot` writes the machine code next to the .wasm; `warp <file>.cwasm` runs it with its imports
#[test]
fn test_compile_aot_writes_machine_code_that_runs() {
	let source = PathBuf::from(OUTPUT_DIRECTORY).join("compile_aot_print.warp");
	fs::write(&source, "print \"hello\"; 6*7").unwrap();
	let machine_code = source.with_extension("cwasm");
	let _ = fs::remove_file(&machine_code);
	let compiled = crate::common::warp_command().args(["compile", "--aot"]).arg(&source).output().unwrap();
	assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
	assert!(source.with_extension("wasm").exists());
	let run = crate::common::warp_command().arg(&machine_code).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.contains("hello") && printed.contains("42"), "{printed}{}", String::from_utf8_lossy(&run.stderr));
}

// card run-wasm-lib: warp's runtime as a C library; a warp program hands it a module's bytes as to any C library
// (`use warp_runtime; run_wasm(bytes)`), the module runs with WASI's fd_write and gives main's result as a text

/// `source` run by warp with libwarp_runtime and its header found: stdout, stderr
fn run_with_runtime_library(source: &str) -> (String, String) {
	let library_path = if cfg!(target_os = "macos") { "DYLD_LIBRARY_PATH" } else { "LD_LIBRARY_PATH" };
	let output = crate::common::warp_command()
		.env(library_path, crate::common::runtime_library_dir())
		.env("WARP_INCLUDE", crate::common::runtime_library_include())
		.arg(source)
		.output()
		.unwrap();
	(String::from_utf8_lossy(&output.stdout).into_owned(), String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn run_wasm_runs_a_module_given_as_bytes() {
	// (module (import "wasi_snapshot_preview1" "fd_write" …) (memory (export "memory") 1) (data (i32.const 16) "hi\n")
	//  (func (export "main") (result i64) fd_write(1, iovec {16, 3}) i64.const 7))
	let source = "use warp_runtime
hi = [0, 97, 115, 109, 1, 0, 0, 0, 1, 13, 2, 96, 4, 127, 127, 127, 127, 1, 127, 96, 0, 1, 126, 2, 35, 1, 22, 119, 97, 115, 105, 95, 115, 110, 97, 112, 115, 104, 111, 116, 95, 112, 114, 101, 118, 105, 101, 119, 49, 8, 102, 100, 95, 119, 114, 105, 116, 101, 0, 0, 3, 2, 1, 1, 5, 3, 1, 0, 1, 7, 17, 2, 6, 109, 101, 109, 111, 114, 121, 2, 0, 4, 109, 97, 105, 110, 0, 1, 10, 31, 1, 29, 0, 65, 0, 65, 16, 54, 2, 0, 65, 4, 65, 3, 54, 2, 0, 65, 1, 65, 0, 65, 1, 65, 8, 16, 0, 26, 66, 7, 11, 11, 9, 1, 0, 65, 16, 11, 3, 104, 105, 10]
print run_wasm(hi)";
	let (stdout, stderr) = run_with_runtime_library(source);
	assert_eq!(stdout, "hi\n7\n", "{stderr}");
}

#[test]
fn run_wasm_of_no_module_fails_loudly() {
	let (stdout, stderr) = run_with_runtime_library("use warp_runtime\nrun_wasm([1, 2, 3])");
	assert!(stderr.contains("run_wasm: "), "{stdout}{stderr}");
}

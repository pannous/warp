//! An installed warp runs `warp pure` / `warp self` with the compilers delivered next to it and never builds one: a
//! missing one is an error naming `warp rebuild` (card installed-compilers, user 2026-10-22)

#[test]
fn a_missing_compiler_is_an_error_naming_rebuild() {
	let folder = crate::common::scratch_directory("installed_compilers");
	let _ = std::fs::remove_dir_all(&folder);
	std::fs::create_dir_all(&folder).unwrap();
	let installed = folder.join("warp");
	std::fs::copy(crate::common::warp_command().get_program(), &installed).unwrap();
	let run = std::process::Command::new(&installed)
		.args(["--no-ask", "pure", "x=6;x*7"])
		.env_remove("WARP_LIB")
		.env("WARP_NO_WINDOW", "1")
		.output()
		.expect("warp runs");
	let said = String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr);
	assert!(said.contains("warp rebuild pure"), "{said}");
	assert!(!folder.join("compiler.wasm").exists(), "built on demand: {said}");
}

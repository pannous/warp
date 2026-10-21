//! One warp build serves every worktree: the warp-written compiler comes from the lib/ at hand (WARP_LIB, else the
//! checkout above the program), not from the checkout that built the binary
const CHECKOUT: &str = "self_hosted_library_checkout";
const PROGRAM: &str = "3 + 4";

fn self_hosted_run(folder: &std::path::Path, library: Option<&std::path::Path>) -> String {
	let mut command = crate::common::warp_command();
	command.current_dir(folder).env("WARP_SELF_HOSTED", "1").env_remove("WARP_LIB").arg("program.warp");
	if let Some(library) = library {
		command.env("WARP_LIB", library);
	}
	let run = command.output().expect("warp runs");
	String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr)
}

/// A checkout whose lib/compiler holds no compiler: reading it fails, naming the file it looked for
fn empty_checkout() -> (std::path::PathBuf, std::path::PathBuf) {
	let checkout = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CHECKOUT);
	let program_folder = checkout.join("probes");
	std::fs::create_dir_all(checkout.join("lib/compiler")).unwrap();
	std::fs::create_dir_all(&program_folder).unwrap();
	std::fs::write(program_folder.join("program.warp"), PROGRAM).unwrap();
	(checkout, program_folder)
}

#[test]
fn the_compiler_comes_from_the_checkout_above_the_program() {
	let (checkout, program_folder) = empty_checkout();
	let output = self_hosted_run(&program_folder, None);
	let expected = checkout.join("lib/compiler/passes.warp");
	assert!(output.contains(&expected.display().to_string()), "{output}");
}

#[test]
fn warp_lib_names_the_compilers_folder() {
	let (checkout, program_folder) = empty_checkout();
	let named = checkout.join("named_lib");
	std::fs::create_dir_all(&named).unwrap();
	let output = self_hosted_run(&program_folder, Some(&named));
	assert!(output.contains(&named.join("compiler/passes.warp").display().to_string()), "{output}");
}

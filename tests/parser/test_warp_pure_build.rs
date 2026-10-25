//! `warp pure|self build <code|file>`: the command words of plain `warp`, the program compiled by compiler.warp (user
//! 2026-10-25: `warp pure build "'hello' + now"` compiled `build 'hello' + now` as code)
use super::test_warp_pure::lane;

/// `warp self build <code>` writes out.wasm in the working folder, as `warp build` does, and the module runs
#[test]
fn warp_self_build_writes_the_module() {
	let folder = crate::common::scratch_directory("warp_self_build");
	let _ = std::fs::remove_dir_all(&folder);
	std::fs::create_dir_all(&folder).unwrap();
	let built = lane("self").args(["build", "'hello' + 1"]).current_dir(&folder).output().expect("warp runs");
	let said = String::from_utf8_lossy(&built.stdout).to_string() + &String::from_utf8_lossy(&built.stderr);
	assert!(built.status.success() && said.contains("to out.wasm"), "{said}");
	let ran = crate::common::warp_command().arg(folder.join("out.wasm")).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&ran.stdout).to_string() + &String::from_utf8_lossy(&ran.stderr);
	assert!(printed.contains("hello1"), "{printed}");
}

/// `warp self run <code>` runs it, as `warp self <code>` does
#[test]
fn warp_self_run_runs_the_code() {
	let ran = lane("self").args(["run", "'hello' + 1"]).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&ran.stdout).to_string() + &String::from_utf8_lossy(&ran.stderr);
	assert!(printed.contains("hello1"), "{printed}");
}

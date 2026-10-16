//! A package's tool is its prebuilt WebAssembly command `<name>.wasm`, run in-process (src/modules/package_tools.rs):
//! warp never builds a package into a shared cargo target directory, where uniscript 1.0.0 once overwrote
//! the binary of a uniscript checkout.

use std::path::Path;
use std::process::Command;
use warp::modules::package_tools::{package_tool, run_package_tool};
use warp::modules::{cached_package, pinned_version};

const WORKSPACE_BOUNDARY_PACKAGE: &str = "packages/workspace-boundary-test";

#[test]
fn a_package_tool_runs_in_the_package_directory() {
	let run = run_package_tool("uniscript", &["<:alpha> <:fracture A>"]).unwrap();
	assert!(run.success(), "{}", run.stderr);
	assert_eq!(run.stdout.trim(), "α 𝔄");
	let check = run_package_tool("uniscript", &["check"]).unwrap(); // reads data/ of the package
	assert!(check.success(), "{}{}", check.stdout, check.stderr);
}

/// Fetched or built once into the package's own build directory, never a cargo target directory shared with other crates
#[test]
fn a_package_tool_lives_in_the_package_build_directory() {
	let tool = package_tool("uniscript").unwrap();
	let pinned_build =
		format!("{}.build", cached_package("uniscript", &pinned_version("uniscript").unwrap()).display());
	let own_directories =
		[Path::new(&pinned_build), Path::new("packages/.build/uniscript"), Path::new("packages/uniscript")];
	assert!(own_directories.iter().any(|directory| tool.parent() == Some(directory)), "{}", tool.display());
	assert!(std::fs::read(&tool).unwrap().starts_with(b"\0asm"));
}

#[test]
fn a_package_without_a_tool_says_so() {
	let failure = run_package_tool("nosuchpackage", &[]).unwrap_err();
	assert!(failure.contains("unknown package: nosuchpackage"), "{failure}");
}

#[test]
fn a_package_source_is_outside_the_warp_workspace() {
	let package = Path::new(WORKSPACE_BOUNDARY_PACKAGE);
	std::fs::remove_dir_all(package).ok();
	std::fs::create_dir_all(package.join("src")).unwrap();
	std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"workspace-boundary-test\"\nversion = \"0.1.0\"\n")
		.unwrap();
	std::fs::write(package.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").unwrap();

	let metadata = Command::new("cargo")
		.args(["metadata", "--offline", "--no-deps", "--format-version", "1", "--manifest-path"])
		.arg(package.join("Cargo.toml"))
		.output()
		.unwrap();
	std::fs::remove_dir_all(package).unwrap();

	assert!(metadata.status.success(), "{}", String::from_utf8_lossy(&metadata.stderr));
}

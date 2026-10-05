//! A package's tool is its prebuilt WebAssembly command `<name>.wasm`, run in-process (src/package_tools.rs):
//! warp never builds a package into a shared cargo target directory, where uniscript 1.0.0 once overwrote
//! the binary of a uniscript checkout.

use std::path::Path;
use warp::modules::{cached_package, pinned_version};
use warp::package_tools::{package_tool, run_package_tool};

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
	let pinned_build = format!("{}.build", cached_package("uniscript", &pinned_version("uniscript").unwrap()).display());
	let own_directories = [Path::new(&pinned_build), Path::new("packages/.build/uniscript"), Path::new("packages/uniscript")];
	assert!(own_directories.iter().any(|directory| tool.parent() == Some(directory)), "{}", tool.display());
	assert!(std::fs::read(&tool).unwrap().starts_with(b"\0asm"));
}

#[test]
fn a_package_without_a_tool_says_so() {
	let failure = run_package_tool("nosuchpackage", &[]).unwrap_err();
	assert!(failure.contains("unknown package: nosuchpackage"), "{failure}");
}

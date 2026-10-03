//! A pinned package (packages.wasp `name: {repository: "git url", version: 1.2.3}`) is the clone of its version's tag,
//! fetched once per machine into ~/.cache/warp/packages and linked as packages/<name>: no fetch per checkout or export.
//! Each test fetches into its own packages directory below scratch/, whatever the checkout's packages/ holds.

use std::path::{Path, PathBuf};
use std::process::Command;
use warp::modules::{fetch_package_into, package_repository};

const PINNED: &str = ".cache/warp/packages/uniscript@1.0.0";

/// An empty packages directory of its own for a test
fn packages_for(test: &str) -> PathBuf {
	let packages = Path::new("scratch/package_pin").join(test);
	std::fs::remove_dir_all(&packages).ok();
	std::fs::create_dir_all(&packages).unwrap();
	packages
}

fn git(directory: &Path, arguments: &[&str]) {
	let status = Command::new("git").arg("-C").arg(directory).args(arguments).status().unwrap();
	assert!(status.success(), "git {arguments:?}");
}

/// packages/uniscript as an unpinned fetch leaves it: a clone whose origin is the registered repository
fn clone_like_a_fetch(test: &str, packages: &Path) -> PathBuf {
	let pinned = fetch_package_into(&packages_for(&format!("{test}-source")), "uniscript").unwrap().canonicalize().unwrap();
	let clone = packages.join("uniscript");
	git(Path::new("."), &["clone", "--quiet", pinned.to_str().unwrap(), clone.to_str().unwrap()]);
	git(&clone, &["remote", "set-url", "origin", &package_repository("uniscript").unwrap()]);
	clone
}

#[test]
fn a_pinned_package_links_its_cached_version() {
	let directory = fetch_package_into(&packages_for("empty"), "uniscript").unwrap();
	let target = std::fs::read_link(&directory).expect("packages/uniscript links into the package cache");
	assert!(target.ends_with(PINNED), "{}", target.display());
	assert!(directory.join("data/entities.idx").is_file()); // the compiled entity index ships with the tag
}

/// What an unpinned warp fetched into packages/ before is moved to packages/.replaced (never deleted) for the pin
#[test]
fn a_clean_unpinned_clone_makes_way_for_the_pin() {
	let packages = packages_for("clean");
	clone_like_a_fetch("clean", &packages);
	let directory = fetch_package_into(&packages, "uniscript").unwrap();
	assert!(std::fs::read_link(&directory).unwrap().ends_with(PINNED));
	let replaced: Vec<_> = std::fs::read_dir(packages.join(".replaced")).unwrap().collect();
	assert_eq!(replaced.len(), 1);
}

/// A clone with changes is someone's work: it stays and wins over the pin (with a warning)
#[test]
fn a_changed_clone_stands_in_for_the_pin() {
	let packages = packages_for("changed");
	let clone = clone_like_a_fetch("changed", &packages);
	std::fs::write(clone.join("uniscript.wasp"), "// local work\n").unwrap();
	let directory = fetch_package_into(&packages, "uniscript").unwrap();
	assert_eq!(directory, clone);
	assert!(std::fs::read_link(&directory).is_err());
	assert!(!packages.join(".replaced").exists());
}

/// The analyzer asks for an FFI signature of every name it meets; the system headers are parsed once per process
#[test]
fn header_signatures_are_parsed_once() {
	crate::requires!(crate::common::MACOS_C_HEADERS);
	let first = warp::ffi::get_signatures_from_headers("m");
	assert!(first.contains_key("sqrt"));
	assert!(std::ptr::eq(first, warp::ffi::get_signatures_from_headers("m")));
	assert!(std::ptr::eq(warp::ffi::get_ffi_signatures(), warp::ffi::get_ffi_signatures()));
}

//! `use <name>` without a local module: registered git repositories (packages.wasp), fetched into packages/<name>

use warp::modules::{fetch_package, package_repository};
use warp::error;
use crate::is;

#[test]
fn the_registry_names_repositories() {
	assert_eq!(package_repository("uniscript").as_deref(), Some("https://github.com/pannous/uniscript.git"));
	assert_eq!(package_repository("nosuchpackage"), None);
}

#[test]
fn a_package_is_fetched_once_into_packages() {
	let directory = fetch_package("uniscript").unwrap();
	assert!(directory.join("data/entities.idx").is_file());
	assert_eq!(fetch_package("uniscript").unwrap(), directory);
}

#[test]
fn use_loads_the_module_of_a_package() {
	is!("use uniscript; uniscript(\"<:alpha>\")", "α");
	is!("use nosuchpackage; 42", error("module not found: nosuchpackage"));
}

/// `module_directory`: the directory of the file it is written in, so a package finds its own files
#[test]
fn module_directory_is_where_the_module_lives() {
	is!("module_directory", ".");
}

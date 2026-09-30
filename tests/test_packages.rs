//! `use package <name>`: registered git repositories (packages.wasp), fetched into packages/<name>

use warp::modules::{fetch_package, package_repository};
use warp::{error, is};

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
fn a_package_module_without_wasp_code_contributes_nothing() {
	is!("use package uniscript; 42", 42);
	is!("use package nosuchpackage; 42", error("unknown package: nosuchpackage"));
}

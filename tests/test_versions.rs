//! Versions: `1.2.3` literals and the soft keyword `version`, compared part by part (wiki: use.md)

use warp::{error, is};

#[test]
fn version_literals_have_two_dots_or_more() {
	is!("x = 1.2.3; x", "1.2.3");
	is!("version 1.10", "1.10");
	is!("1.5", 1.5);
	assert_eq!(warp::parse_data("version: 1.2.3").serialize(), "version:1.2.3");
}

#[test]
fn versions_compare_part_by_part() {
	is!("version 1.9 < version 1.10", true);
	is!("1.2.3 < 1.10.0", true);
	is!("1.2.0 == 1.2", true);
	is!("version 2.0.1 >= version 2.1", false);
}

#[test]
fn version_is_a_soft_keyword() {
	is!("version = 2; version + 1", 3);
}

#[test]
fn use_requires_a_version_of_a_package() {
	is!("use uniscript >= 1.0; uniscript(\"<:alpha>\")", "α");
	is!("use uniscript from 0.1.5; uniscript(\"<:alpha>\")", "α");
	is!("use uniscript version 1.0.0; uniscript(\"<:alpha>\")", "α");
	is!("use uniscript >= 9.0.0; 1", error("package uniscript has no version >= 9.0.0 (tagged: 0.1.0, 1.0.0)"));
}

/// a version the default branch does not have comes from the git tag `v0.1.0`, into packages/uniscript@0.1.0
#[test]
fn an_older_version_comes_from_its_git_tag() {
	is!("use uniscript version 0.1.0; 42", 42);
	assert!(std::path::Path::new("packages/uniscript@0.1.0/Cargo.toml").is_file());
}

/// `v1.2.3`, as git tags name versions; `v2` stays a name
#[test]
fn tagged_version_literals() {
	is!("v1.2.3", "1.2.3");
	is!("v1.2.3 < 1.10.0", true);
	is!("v2 = 7; v2", 7);
	is!("use uniscript >= v1.0.0; uniscript(\"<:alpha>\")", "α");
}

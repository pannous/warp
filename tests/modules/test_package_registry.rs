//! Card build-sh / package registry (user 2026-10-17): most packages need no registry line. The default registry
//! github.com/pannous/warp-packages holds one folder per package (`use colors`), its versions tagged `colors@1.0.0`;
//! a folder whose manifest package.warp names a `repository` is a proxy for it (`use uniscript`). A packages.warp line
//! only pins or redirects: `name: 1.2.3`, `name: "owner/repo@1.2.3"`, the long `{repository, version, library}`.

use crate::is;
use std::path::Path;
use warp::modules::{
	fetch_package_into, fetch_package_version, package_in, package_repository, pinned_version, DEFAULT_REGISTRY,
};
use warp::versions::{Requirement, Version};
use warp::Op;

const GITHUB_UNISCRIPT: &str = "https://github.com/pannous/uniscript.git";

/// The url, folder and pinned version an entry of a registry text resolves to, before any manifest is read
fn entry(registry: &str, name: &str) -> (String, Option<String>, Option<String>) {
	let package = package_in(registry, name).unwrap_or_else(|| panic!("no {name} in {registry}")).unwrap();
	(package.url, package.folder, package.pinned.map(|version| version.to_string()))
}

fn in_default_registry(registry: &str, pinned: Option<&str>) {
	let expected = (DEFAULT_REGISTRY.to_string(), Some("colors".to_string()), pinned.map(str::to_string));
	assert_eq!(entry(registry, "colors"), expected, "{registry}");
}

#[test]
fn a_bare_name_or_version_is_the_default_registrys_folder() {
	in_default_registry("colors", None);
	in_default_registry("colors: latest", None);
	in_default_registry("colors: 1.0.0", Some("1.0.0"));
	in_default_registry("colors: \"1.0.0\"", Some("1.0.0"));
	in_default_registry("colors: {version: 1.0.0}", Some("1.0.0"));
}

#[test]
fn a_repository_takes_its_version_after_an_at() {
	let pinned = (GITHUB_UNISCRIPT.to_string(), None, Some("1.0.10".to_string()));
	assert_eq!(entry("uniscript: \"pannous/uniscript@1.0.10\"", "uniscript"), pinned);
	assert_eq!(entry("uniscript: {repository: pannous/uniscript, version: 1.0.10}", "uniscript"), pinned);
	let ssh = "uniscript: \"git@github.com:pannous/uniscript.git\"";
	assert_eq!(entry(ssh, "uniscript").0, "git@github.com:pannous/uniscript.git"); // git@ is no version
}

/// uniscript has no line in packages.warp: its proxy in the default registry names the repository and its version
#[test]
fn a_proxy_is_the_repository_it_names() {
	assert_eq!(package_repository("uniscript").as_deref(), Some(GITHUB_UNISCRIPT));
	assert!(pinned_version("uniscript").is_some());
	is!("use uniscript; uniscript(\"<:alpha/>\")", "α"); // through the proxy's binding of c/ffi/uniscript_c.wasm
}

/// A folder package: its latest is the default branch's folder, linked from packages/
#[cfg_attr(not(feature = "native"), ignore = "browser: needs git")]
#[test]
fn a_folder_package_is_fetched_by_name() {
	assert_eq!(package_repository("colors").as_deref(), Some(DEFAULT_REGISTRY));
	let packages = Path::new("scratch/package_registry");
	std::fs::remove_dir_all(packages).ok();
	let directory = fetch_package_into(packages, "colors").unwrap();
	assert!(directory.join("colors.warp").is_file());
	is!("use colors; to_hex(from_hex(\"f80\"))", "#ff8800");
}

/// A version of a folder package is its tag `<name>@<version>`, linked as packages/<name>@<version>; `use colors
/// version 1.0.0` takes it when the default branch declares another version
#[cfg_attr(not(feature = "native"), ignore = "browser: needs git")]
#[test]
fn a_folder_package_version_is_its_tag() {
	let exactly = Requirement(Op::Eq, Version::parse("1.0.0").unwrap());
	let directory = fetch_package_version("colors", &exactly).unwrap();
	let target = std::fs::read_link(&directory).unwrap();
	assert!(target.ends_with("colors@1.0.0/colors"), "{}", target.display());
	assert!(directory.join("colors.warp").is_file());
	is!("use colors version 1.0.0; to_hex(from_hex(\"#ff8800\"))", "#ff8800");
}

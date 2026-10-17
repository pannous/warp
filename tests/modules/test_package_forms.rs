//! Card packages (user): a registry entry is as short as it can be. `name/repo` and `github:name/repo` are GitHub
//! repositories, a bare name the default registry's folder (user 2026-10-17, test_package_registry.rs);
//! `version: latest`, or no version, follows the default branch.

use warp::modules::{package_in, registry_entries, DEFAULT_REGISTRY, PACKAGE_REGISTRY};

const GITHUB_UNISCRIPT: &str = "https://github.com/pannous/uniscript.git";

fn resolves(registry: &str, url: &str, pinned: Option<&str>) {
	let package = package_in(registry, "uniscript").unwrap_or_else(|| panic!("no uniscript in {registry}")).unwrap();
	assert_eq!(package.url, url, "{registry}");
	assert_eq!(package.pinned.map(|version| version.to_string()).as_deref(), pinned, "{registry}");
}

#[test]
fn a_bare_name_is_a_folder_of_the_default_registry() {
	resolves("uniscript", DEFAULT_REGISTRY, None);
	resolves("other: \"x\"\nuniscript\n", DEFAULT_REGISTRY, None);
}

#[test]
fn an_owner_and_repository_are_on_github() {
	resolves("uniscript: pannous/uniscript", GITHUB_UNISCRIPT, None);
	resolves("uniscript: \"pannous/uniscript\"", GITHUB_UNISCRIPT, None);
	resolves("uniscript: \"github:pannous/uniscript.git\"", GITHUB_UNISCRIPT, None);
	resolves("uniscript: \"github:/pannous/uniscript\"", GITHUB_UNISCRIPT, None);
}

#[test]
fn version_latest_follows_the_default_branch() {
	resolves("uniscript: {repository: \"github:pannous/uniscript.git\", version: latest}", GITHUB_UNISCRIPT, None);
	resolves("uniscript: {repository: \"pannous/uniscript\"}", GITHUB_UNISCRIPT, None);
	resolves("uniscript: {repository: \"pannous/uniscript\", version: 1.0.7}", GITHUB_UNISCRIPT, Some("1.0.7"));
}

#[test]
fn a_full_url_stays_as_written() {
	resolves("uniscript: \"https://example.org/u.git\"", "https://example.org/u.git", None);
	resolves("uniscript: \"git@github.com:pannous/uniscript.git\"", "git@github.com:pannous/uniscript.git", None);
	resolves("uniscript: \"/Users/me/dev/uniscript\"", "/Users/me/dev/uniscript", None);
}

fn refuses(registry: &str, needle: &str) {
	match package_in(registry, "uniscript") {
		Some(Err(message)) => assert!(message.contains(needle), "{message}"),
		other => {
			panic!("expected an error containing {needle:?} for {registry}, got {:?}", other.map(|p| p.map(|p| p.url)))
		}
	}
}

// review of fix-package-forms: a misspelt version or an unreadable repository is an error naming the entry, never a
// silent unpin or a missing package
#[test]
fn an_unreadable_version_is_an_error() {
	refuses("uniscript: {repository: pannous/uniscript, version: lastest}", "version lastest");
	refuses("uniscript: {repository: pannous/uniscript, version: \"1.0.x\"}", "is no version");
}

#[test]
fn unquoted_repository_names_with_dashes_and_dots_resolve() {
	resolves("uniscript: pannous/my-repo", "https://github.com/pannous/my-repo.git", None);
	resolves("uniscript: pannous/repo.js", "https://github.com/pannous/repo.js.git", None);
	refuses("uniscript: pannous/2fa", "quote it");
}

#[test]
fn every_registry_entry_resolves() {
	for (name, package) in registry_entries(PACKAGE_REGISTRY) {
		assert!(package.is_ok(), "packages.warp {name}: {:?}", package.err());
	}
}

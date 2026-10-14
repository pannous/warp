//! Card packages (user): a registry entry is as short as it can be. `name/repo`, `github:name/repo` and a bare name
//! (owner pannous) are GitHub repositories; `version: latest`, or no version, follows the default branch.

use warp::modules::package_in;

const GITHUB_UNISCRIPT: &str = "https://github.com/pannous/uniscript.git";

fn resolves(registry: &str, url: &str, pinned: Option<&str>) {
	let package = package_in(registry, "uniscript").unwrap_or_else(|| panic!("no uniscript in {registry}"));
	assert_eq!(package.url, url, "{registry}");
	assert_eq!(package.pinned.map(|version| version.to_string()).as_deref(), pinned, "{registry}");
}

#[test]
fn a_bare_name_is_a_repository_of_pannous() {
	resolves("uniscript", GITHUB_UNISCRIPT, None);
	resolves("other: \"x\"\nuniscript\n", GITHUB_UNISCRIPT, None);
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

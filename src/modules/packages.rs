//! Packages of the registry: fetched once per pinned version as shallow clones, local checkouts winning, and module versions
use super::*;

/// name: "git url" for every package, compiled in so warp finds it from any directory
pub(crate) const PACKAGE_REGISTRY: &str = include_str!("../../packages.warp");
/// where packages are fetched to, below the working directory
pub const PACKAGES_DIRECTORY: &str = "packages";
/// where a tagged version of a package is fetched once per machine and linked from packages/: a tag never changes
pub(crate) const PACKAGE_CACHE: &str = ".cache/warp/packages";
/// where an unpinned clone in packages/ goes when a pin takes its place
pub(crate) const REPLACED_DIRECTORY: &str = ".replaced";
/// one fetch at a time within a process; parallel processes clone to their own staging directory and rename
pub(crate) static FETCHING: Mutex<()> = Mutex::new(());

/// A package of the registry: `name: "git url"` follows the default branch,
/// `name: {repository: "git url", version: 1.2.3}` is pinned to the git tag of that version
pub(super) struct Registered {
	pub(super) url: String,
	pub(super) pinned: Option<Version>,
}

pub(super) fn registered(name: &str) -> Option<Registered> {
	statements(WarpParser::parse(PACKAGE_REGISTRY)).iter().find_map(|entry| {
		let Node::Key(key, _, value) = entry.drop_meta() else {
			return None;
		};
		if !matches!(key.drop_meta(), Node::Symbol(key) if key == name) {
			return None;
		}
		match value.drop_meta() {
			Node::Text(url) => Some(Registered { url: url.clone(), pinned: None }),
			pinned => match pinned["repository"].drop_meta() {
				Node::Text(url) => Some(Registered { url: url.clone(), pinned: Some(version_of(&pinned["version"])?) }),
				_ => None,
			},
		}
	})
}

/// The git url of a package in the registry
pub fn package_repository(name: &str) -> Option<String> {
	registered(name).map(|package| package.url)
}

/// The version a package is pinned to in the registry, None for one that follows its default branch
pub fn pinned_version(name: &str) -> Option<Version> {
	registered(name).and_then(|package| package.pinned)
}

/// Where the pinned version of a package is cloned once per machine: ~/.cache/warp/packages/<name>@<version>
pub fn cached_package(name: &str, version: &Version) -> PathBuf {
	package_cache().join(format!("{name}@{version}"))
}

/// The directory of a package, packages/<name>: the pinned version, or a clone of the default branch;
/// whatever is there already stays, so a local checkout linked there stands in for the package
pub fn fetch_package(name: &str) -> Result<PathBuf, String> {
	fetch_package_into(Path::new(PACKAGES_DIRECTORY), name)
}

/// fetch_package with another packages directory than packages/
pub fn fetch_package_into(packages: &Path, name: &str) -> Result<PathBuf, String> {
	let package = registered_package(name)?;
	match &package.pinned {
		Some(version) => fetch_tagged(packages, name, &package.url, version, name),
		None => clone(name, &package.url, &packages.join(name), None),
	}
}

/// The directory of the version of a package a requirement picks among its git tags (`v1.2.3` or `1.2.3`):
/// that version, or the latest from the minimum on; cloned into packages/<name>@<version>
pub fn fetch_package_version(name: &str, requirement: &Requirement) -> Result<PathBuf, String> {
	let url = registered_package(name)?.url;
	let tags = package_version_tags(name, &url)?;
	let Some((_, version)) =
		tags.iter().filter(|(_, version)| requirement.allows(version)).max_by(|a, b| a.1.cmp(&b.1))
	else {
		let versions: Vec<String> = tags.iter().map(|(_, version)| version.to_string()).collect();
		return Err(format!("package {name} has no {requirement} (tagged: {})", versions.join(", ")));
	};
	fetch_tagged(Path::new(PACKAGES_DIRECTORY), name, &url, version, &format!("{name}@{version}"))
}

/// packages/<link_name>, a link to the version's shallow clone in the machine's package cache: fetched once per machine,
/// never again per checkout, export or test run. A link into the cache to another version (the pin moved) or to a
/// removed clone is replaced. A clean clone of the repository there (an unpinned fetch) moves to packages/.replaced;
/// anything else there (a local checkout, a clone with changes) stands in for the pin, with a warning.
pub(super) fn fetch_tagged(
	packages: &Path,
	name: &str,
	url: &str,
	version: &Version,
	link_name: &str,
) -> Result<PathBuf, String> {
	pub(super) static LINKING: Mutex<()> = Mutex::new(()); // clone() takes FETCHING itself
	let _linking = LINKING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let cached = cached_package(name, version);
	let link = packages.join(link_name);
	let stale = |target: PathBuf| target.starts_with(package_cache()) && (target != cached || !cached.exists());
	if std::fs::read_link(&link).is_ok_and(stale) {
		std::fs::remove_file(&link)
			.map_err(|failure| format!("package {name}: stale link {}: {failure}", link.display()))?;
	}
	if is_clean_clone(&link, url) {
		set_aside(packages, name, version, &link)?;
	}
	if link.exists() {
		if std::fs::read_link(&link).map_or(true, |target| target != cached) {
			warn_overridden(name, version, &link);
		}
		return Ok(link);
	}
	if !cached.exists() {
		let tags = package_version_tags(name, url)?;
		let tag = tags
			.iter()
			.find(|(_, tagged)| tagged == version)
			.map(|(tag, _)| tag.as_str())
			.ok_or(format!("package {name} has no tag of version {version} at {url}"))?;
		clone(name, url, &cached, Some(tag))?;
	}
	make_package_directory(name, packages)?;
	#[cfg(not(unix))]
	return Err(format!("package {name}: not linked to {}: no symbolic links on this platform", cached.display()));
	#[cfg(unix)]
	match std::os::unix::fs::symlink(&cached, &link) {
		Err(failure) if !link.exists() => Err(format!("package {name}: not linked to {}: {failure}", cached.display())),
		_ => Ok(link),
	}
}

/// A directory (not a link) holding a git clone of url without any change: what an unpinned fetch left there
pub(super) fn is_clean_clone(directory: &Path, url: &str) -> bool {
	let git = |arguments: &[&str]| {
		Command::new("git")
			.arg("-C")
			.arg(directory)
			.args(arguments)
			.output()
			.ok()
			.filter(|output| output.status.success())
			.map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
	};
	is_real_directory(directory)
		&& directory.join(".git").exists()
		&& git(&["remote", "get-url", "origin"]).as_deref() == Some(url)
		&& git(&["status", "--porcelain"]).is_some_and(|changes| changes.is_empty())
}

pub(super) fn is_real_directory(path: &Path) -> bool {
	std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Moves an unpinned clone out of the pin's way into packages/.replaced, never deleting it
pub(super) fn set_aside(packages: &Path, name: &str, version: &Version, clone: &Path) -> Result<(), String> {
	let replaced = packages.join(REPLACED_DIRECTORY);
	make_package_directory(name, &replaced)?;
	let seconds =
		std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
	let destination = replaced.join(format!(
		"{}.{seconds}.{}",
		clone.file_name().unwrap_or_default().to_string_lossy(),
		std::process::id()
	));
	match std::fs::rename(clone, &destination) {
		Ok(()) => eprintln!(
			"package {name}: {} was an unpinned clone, moved to {}; now the pinned {version}",
			clone.display(),
			destination.display()
		),
		Err(_) if !is_real_directory(clone) => {} // another process moved it first
		Err(failure) => {
			return Err(format!("package {name}: {} not moved out of the pin's way: {failure}", clone.display()));
		}
	}
	Ok(())
}

/// A local checkout or a changed clone in packages/ wins over the pin, loudly, once per package and process
pub(super) fn warn_overridden(name: &str, version: &Version, directory: &Path) {
	pub(super) static WARNED: Mutex<Vec<String>> = Mutex::new(Vec::new());
	let mut warned = WARNED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let key = directory.display().to_string();
	if !warned.contains(&key) {
		eprintln!(
			"warning: package {name}: {key} (a local checkout or a clone with changes) overrides the pinned version {version}"
		);
		warned.push(key);
	}
}

pub(super) fn registered_package(name: &str) -> Result<Registered, String> {
	registered(name).ok_or(format!("unknown package: {name}"))
}

pub(super) fn package_version_tags(name: &str, url: &str) -> Result<Vec<(String, Version)>, String> {
	version_tags(url).map_err(|failure| format!("package {name}: no versions from {url}: {failure}"))
}

pub(super) fn make_package_directory(name: &str, directory: &Path) -> Result<(), String> {
	std::fs::create_dir_all(directory).map_err(|failure| format!("package {name}: {failure}"))
}

pub(super) fn package_cache() -> PathBuf {
	PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(PACKAGE_CACHE)
}

/// The tags of a repository that name versions, with their versions
pub(super) fn version_tags(url: &str) -> Result<Vec<(String, Version)>, String> {
	let listing = Command::new("git")
		.args(["ls-remote", "--tags", "--refs", url])
		.output()
		.map_err(|failure| failure.to_string())?;
	if !listing.status.success() {
		return Err(String::from_utf8_lossy(&listing.stderr).trim().to_string());
	}
	Ok(String::from_utf8_lossy(&listing.stdout)
		.lines()
		.filter_map(|line| line.split("refs/tags/").nth(1))
		.filter_map(|tag| Some((tag.to_string(), Version::parse(tag)?)))
		.collect())
}

/// A shallow clone of a repository, of a tag or the default branch, into directory unless it is there
pub(super) fn clone(name: &str, url: &str, directory: &Path, tag: Option<&str>) -> Result<PathBuf, String> {
	let _fetching = FETCHING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if directory.exists() {
		return Ok(directory.to_path_buf());
	}
	let parent = directory.parent().unwrap_or(Path::new("."));
	make_package_directory(name, parent)?;
	let directory_name = directory.file_name().unwrap_or_default().to_string_lossy();
	let staging = parent.join(format!(".{directory_name}.fetching.{}", std::process::id()));
	let mut command = Command::new("git");
	command.args(["clone", "--quiet", "--depth", "1"]);
	if let Some(tag) = tag {
		command.args(["--branch", tag]);
	}
	let clone = command
		.arg(url)
		.arg(&staging)
		.output()
		.map_err(|failure| format!("package {name} not fetched: git: {failure}"))?;
	if !clone.status.success() {
		return Err(format!(
			"package {name} not fetched from {url}: {}",
			String::from_utf8_lossy(&clone.stderr).trim()
		));
	}
	if std::fs::rename(&staging, directory).is_err() && directory.exists() {
		std::fs::remove_dir_all(&staging).ok(); // another process fetched it first
	}
	Ok(directory.to_path_buf())
}

pub(super) fn package_module(directory: &Path, name: &str) -> Option<PathBuf> {
	MODULE_EXTENSIONS
		.iter()
		.map(|extension| directory.join(format!("{name}.{extension}")))
		.find(|path| module_exists(path))
}

pub(super) fn module_exists(path: &Path) -> bool {
	crate::web::file_exists(&path.to_string_lossy())
}

/// Where a registered package's files are: its fetched clone, or in the browser the raw files of its repository at the
/// pinned tag (GitHub serves them to any page) or default branch
pub(super) fn package_directory(name: &str) -> Result<PathBuf, String> {
	#[cfg(feature = "native")]
	return fetch_package(name);
	#[cfg(not(feature = "native"))]
	{
		let package = registered_package(name)?;
		let repository =
			package.url.trim_end_matches(".git").replace("https://github.com/", "https://raw.githubusercontent.com/");
		let reference = package.pinned.map_or("HEAD".to_string(), |version| format!("v{version}"));
		Ok(PathBuf::from(format!("{repository}/{reference}")))
	}
}

/// The version a module file declares with a top level `version 1.2.3`
pub(super) fn module_version(path: &Path) -> Option<Version> {
	let source = crate::web::read_text(&path.to_string_lossy())?;
	// reading the version is no reading of the module's code: its style gets no hints (card hints-fire)
	crate::normalize::without_hints(|| declared_version(&statements(WarpParser::parse(&source))))
}

/// A local module a `use … version` names must declare a version the requirement allows
pub(super) fn check_version(name: &str, path: &Path, requirement: &Requirement) -> Result<(), Node> {
	match module_version(path) {
		Some(version) if requirement.allows(&version) => Ok(()),
		Some(version) => Err(error(&format!("module {name} is version {version}, {requirement} required"))),
		None => Err(error(&format!("module {name} declares no version, {requirement} required"))),
	}
}

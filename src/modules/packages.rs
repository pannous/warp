//! Packages: an entry of packages.warp, else a folder of the default registry warp-packages; fetched once per pinned
//! version as shallow clones, local checkouts winning, and module versions (wiki/Packages.md)
use super::*;

/// name: "git url" for every package, compiled in so warp finds it from any directory
pub const PACKAGE_REGISTRY: &str = include_str!("../../packages.warp");
/// The default registry: one folder per package, `<name>/<name>.warp` with its manifest `<name>/package.warp`, its
/// versions tagged `<name>@1.2.3`. A manifest naming a `repository` makes the folder a proxy for that repository.
pub const DEFAULT_REGISTRY: &str = "https://github.com/pannous/warp-packages.git";
/// A package's manifest: the keys of a long packages.warp entry (repository, version, library)
pub const PACKAGE_MANIFEST: &str = "package.warp";
/// the default registry's default branch, cloned into the package cache
const REGISTRY_CLONE: &str = ".registry";
/// how long the clone of the default registry stands for its latest state
#[cfg(feature = "native")]
const REGISTRY_REFRESH: std::time::Duration = std::time::Duration::from_secs(60 * 60);
/// a folder package's version tags: `colors@1.2.3`
const FOLDER_TAG_SEPARATOR: char = '@';
/// where packages are fetched to, below the working directory
pub const PACKAGES_DIRECTORY: &str = "packages";
/// where a tagged version of a package is fetched once per machine and linked from packages/: a tag never changes
pub(crate) const PACKAGE_CACHE: &str = ".cache/warp/packages";
/// where an unpinned clone in packages/ goes when a pin takes its place
pub(crate) const REPLACED_DIRECTORY: &str = ".replaced";
/// one fetch at a time within a process; parallel processes clone to their own staging directory and rename
pub(crate) static FETCHING: Mutex<()> = Mutex::new(());

/// The owner of a package written as a bare name (card packages, user: `uniscript` just works)
pub(crate) const DEFAULT_PACKAGE_OWNER: &str = "pannous";
const GITHUB_PREFIX: &str = "github:";
/// `version: latest` follows the default branch, as no version does
const LATEST_VERSION: &str = "latest";

/// A package: `name: "git url"` follows the default branch, `name: {repository: "git url", version: 1.2.3}` is pinned
/// to the git tag of that version. Short forms: `name: 1.2.3` (the default registry's package, pinned),
/// `name: "owner/repo@1.2.3"`, `name: owner/repo`, `name: "github:owner/repo"`, `name: latest`; no entry at all is the
/// default registry's latest. `library: "x"` (in an entry or a manifest): warp calls the package's prebuilt core module
/// x.wasm (header x.h beside it) through the binding `<name>.warp` of the folder declaring it, loaded beside the library
pub struct Registered {
	pub url: String,
	pub pinned: Option<Version>,
	pub library: Option<String>,
	/// the package's folder in the default registry, whose versions are tagged `<folder>@1.2.3`
	pub folder: Option<String>,
	/// a proxy's folder in the default registry, which binds the library of the repository it names
	pub proxy: Option<PathBuf>,
}

/// A package by its name: the entry of packages.warp, else the default registry's folder, a proxy resolved to the
/// repository it names
pub(super) fn registered(name: &str) -> Option<Result<Registered, String>> {
	let entry = package_in(PACKAGE_REGISTRY, name);
	if entry.is_none() && registry_folder(name).is_none() {
		return None;
	}
	Some(entry.unwrap_or_else(|| Ok(in_default_registry(name))).and_then(|package| through_manifest(name, package)))
}

/// A package of the default registry as its manifest describes it: a proxy is the repository it names, a version
/// written in packages.warp winning over the proxy's
fn through_manifest(name: &str, package: Registered) -> Result<Registered, String> {
	if package.folder.is_none() {
		return Ok(package);
	}
	let folder = registry_folder(name).ok_or(format!("package {name}: no folder {name} in {DEFAULT_REGISTRY}"))?;
	let manifest = crate::web::read_text(&folder.join(PACKAGE_MANIFEST).to_string_lossy()).unwrap_or_default();
	let fields = Node::List(statements(WarpParser::parse(&manifest)), Bracket::Curly, Separator::Newline);
	let described = package_of(name, &fields["repository"], &fields["version"], &fields["library"])
		.map_err(|failure| format!("{failure} ({DEFAULT_REGISTRY} {name}/{PACKAGE_MANIFEST})"))?;
	Ok(match described.folder {
		Some(_) => Registered { library: described.library, ..package },
		None => Registered { pinned: package.pinned.or(described.pinned), proxy: Some(folder), ..described },
	})
}

fn in_default_registry(name: &str) -> Registered {
	Registered {
		url: DEFAULT_REGISTRY.to_string(),
		pinned: None,
		library: None,
		folder: Some(name.to_string()),
		proxy: None,
	}
}

/// The package `name` of a registry text written like packages.warp; an error for an entry that names no repository
/// or no version warp can read
pub fn package_in(registry: &str, name: &str) -> Option<Result<Registered, String>> {
	registry_entries(registry).into_iter().find(|(key, _)| key == name).map(|(_, package)| package)
}

/// Every entry of a registry text, by name
pub fn registry_entries(registry: &str) -> Vec<(String, Result<Registered, String>)> {
	statements(WarpParser::parse(registry))
		.iter()
		.filter_map(|entry| match entry.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), Ok(in_default_registry(name)))),
			Node::Key(key, _, value) => match key.drop_meta() {
				Node::Symbol(name) => Some((name.clone(), registry_entry(name, value))),
				_ => None,
			},
			_ => None,
		})
		.collect()
}

fn registry_entry(name: &str, value: &Node) -> Result<Registered, String> {
	match value.drop_meta() {
		Node::List(_, Bracket::Curly, _) => {
			package_of(name, &value["repository"], &value["version"], &value["library"])
		}
		_ if value.is_symbol(LATEST_VERSION) || version_of(value).is_some() => {
			package_of(name, &Node::Empty, value, &Node::Empty)
		}
		_ => package_of(name, value, &Node::Empty, &Node::Empty),
	}
}

/// The package the fields of an entry or a manifest describe; no repository is the default registry's folder
fn package_of(name: &str, repository: &Node, version: &Node, library: &Node) -> Result<Registered, String> {
	let mut pinned = match version.drop_meta() {
		Node::Empty => None,
		Node::Symbol(word) if word == LATEST_VERSION => None,
		written => Some(version_of(written).ok_or(format!(
			"package {name}: version {} is no version: write 1.2.3 or {LATEST_VERSION}",
			written.serialize().trim()
		))?),
	};
	let mut package = match repository.drop_meta() {
		Node::Empty => in_default_registry(name),
		written => {
			let path = path_of(written).ok_or(format!(
				"package {name}: no repository in {}: quote it, \"owner/repository\"",
				written.serialize().trim()
			))?;
			let (path, tagged) = split_version(&path);
			pinned = pinned.or(tagged);
			Registered { url: repository_url(path), pinned: None, library: None, folder: None, proxy: None }
		}
	};
	package.pinned = pinned;
	package.library = match library.drop_meta() {
		Node::Text(library) => Some(library.clone()),
		_ => None,
	};
	Ok(package)
}

/// `owner/repo@1.2.3` → (`owner/repo`, 1.2.3); an ssh address's `git@` is no version
fn split_version(path: &str) -> (&str, Option<Version>) {
	match path.rsplit_once('@').and_then(|(repository, version)| Some((repository, Version::parse(version)?))) {
		Some((repository, version)) => (repository, Some(version)),
		None => (path, None),
	}
}

/// The git url of a written repository: a url, ssh address or path stays; `github:owner/repo`, `owner/repo` and a
/// bare `repo` (of DEFAULT_PACKAGE_OWNER) are GitHub repositories
fn repository_url(written: &str) -> String {
	let is_url = written.contains("://") || written.starts_with("git@") || written.starts_with(['/', '.', '~']);
	if is_url {
		return written.to_string();
	}
	let path = written.strip_prefix(GITHUB_PREFIX).unwrap_or(written).trim_start_matches('/').trim_end_matches(".git");
	let path = if path.contains('/') { path.to_string() } else { format!("{DEFAULT_PACKAGE_OWNER}/{path}") };
	format!("https://github.com/{path}.git")
}

/// The git url of a package in the registry
pub fn package_repository(name: &str) -> Option<String> {
	registered(name)?.ok().map(|package| package.url)
}

/// The version a package is pinned to in the registry, None for one that follows its default branch
pub fn pinned_version(name: &str) -> Option<Version> {
	registered(name)?.ok().and_then(|package| package.pinned)
}

/// Is `name` a package warp knows without fetching anything: an entry of packages.warp or a folder of the default
/// registry's clone as it is
pub(super) fn known_package(name: &str) -> bool {
	package_in(PACKAGE_REGISTRY, name).is_some() || cfg!(feature = "native") && registry_clone().join(name).is_dir()
}

/// The folder of a package in the default registry: in its clone, refreshed when older than REGISTRY_REFRESH (a missing
/// folder does not force a fetch: `use sqlite3`, a C library, asks the registry at most that often)
#[cfg(feature = "native")]
fn registry_folder(name: &str) -> Option<PathBuf> {
	if !is_package_name(name) {
		return None;
	}
	Some(current_registry().join(name)).filter(|folder| folder.is_dir())
}

/// The clone of the default registry, refreshed when older than REGISTRY_REFRESH
#[cfg(feature = "native")]
fn current_registry() -> PathBuf {
	let registry = registry_clone();
	if is_stale(&registry) {
		refresh_registry(&registry);
	}
	registry
}

/// In the browser: the raw files of the default registry's default branch, a folder being there when its manifest is
#[cfg(not(feature = "native"))]
fn registry_folder(name: &str) -> Option<PathBuf> {
	let folder = PathBuf::from(format!("{}/HEAD/{name}", raw_repository(DEFAULT_REGISTRY)));
	(is_package_name(name) && module_exists(&folder.join(PACKAGE_MANIFEST))).then_some(folder)
}

/// A name that is one folder: no path, no hidden file
fn is_package_name(name: &str) -> bool {
	!name.is_empty() && !name.starts_with('.') && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn registry_clone() -> PathBuf {
	package_cache().join(REGISTRY_CLONE)
}

#[cfg(feature = "native")]
fn is_stale(registry: &Path) -> bool {
	let fetched =
		["FETCH_HEAD", "HEAD"].iter().find_map(|file| std::fs::metadata(registry.join(".git").join(file)).ok());
	fetched.and_then(|metadata| metadata.modified().ok()?.elapsed().ok()).is_none_or(|age| age > REGISTRY_REFRESH)
}

/// The latest state of the default registry: cloned, or its default branch fetched; once per process, loudly failing
#[cfg(feature = "native")]
fn refresh_registry(registry: &Path) {
	static REFRESHED: Mutex<bool> = Mutex::new(false);
	let mut refreshed = REFRESHED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if std::mem::replace(&mut *refreshed, true) {
		return;
	}
	let refresh = if registry.exists() {
		git_in(registry, &["fetch", "--quiet", "--depth", "1", "origin", "HEAD"])
			.and_then(|_| git_in(registry, &["checkout", "--quiet", "--force", "--detach", "FETCH_HEAD"]))
			.map(|_| ())
	} else {
		clone("registry", DEFAULT_REGISTRY, registry, None).map(|_| ())
	};
	if let Err(failure) = refresh {
		eprintln!("warning: the default registry {DEFAULT_REGISTRY} not refreshed: {failure}");
	}
}

/// What git printed, or what it complained about
fn git_in(directory: &Path, arguments: &[&str]) -> Result<String, String> {
	let output = Command::new("git")
		.arg("-C")
		.arg(directory)
		.args(arguments)
		.output()
		.map_err(|failure| format!("git: {failure}"))?;
	match output.status.success() {
		true => Ok(String::from_utf8_lossy(&output.stdout).trim().to_string()),
		false => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
	}
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

/// The named packages fetched into packages/; none named: every package of packages.warp and the default registry
#[cfg(feature = "native")]
pub fn fetch_packages(names: &[String]) -> Vec<(String, Result<PathBuf, String>)> {
	let every_package = || {
		let folders = std::fs::read_dir(current_registry()).into_iter().flatten().flatten().map(|entry| entry.path());
		let folders =
			folders.filter(|path| path.is_dir()).filter_map(|path| Some(path.file_name()?.to_str()?.to_string()));
		let entries = registry_entries(PACKAGE_REGISTRY).into_iter().map(|(name, _)| name);
		let mut names: Vec<String> = entries.chain(folders.filter(|name| is_package_name(name))).collect();
		names.sort();
		names.dedup();
		names
	};
	let names = if names.is_empty() { every_package() } else { names.to_vec() };
	names.into_iter().map(|name| (name.clone(), fetch_package(&name))).collect()
}

/// fetch_package with another packages directory than packages/
pub fn fetch_package_into(packages: &Path, name: &str) -> Result<PathBuf, String> {
	let package = registered_package(name)?;
	match (&package.pinned, &package.folder) {
		(Some(version), _) => fetch_tagged(packages, name, &package, version, name),
		(None, Some(folder)) => {
			let latest =
				registry_folder(folder).ok_or(format!("package {name}: no folder {folder} in {DEFAULT_REGISTRY}"))?;
			link_package(packages, name, name, &latest)
		}
		(None, None) => clone(name, &package.url, &packages.join(name), None),
	}
}

/// The directory of the version of a package a requirement picks among its git tags (`v1.2.3` or `1.2.3`, a folder
/// package's `<name>@1.2.3`): that version, or the latest from the minimum on; linked as packages/<name>@<version>
pub fn fetch_package_version(name: &str, requirement: &Requirement) -> Result<PathBuf, String> {
	let package = registered_package(name)?;
	let tags = package_version_tags(name, &package)?;
	let Some((_, version)) =
		tags.iter().filter(|(_, version)| requirement.allows(version)).max_by(|a, b| a.1.cmp(&b.1))
	else {
		let versions: Vec<String> = tags.iter().map(|(_, version)| version.to_string()).collect();
		return Err(format!("package {name} has no {requirement} (tagged: {})", versions.join(", ")));
	};
	fetch_tagged(Path::new(PACKAGES_DIRECTORY), name, &package, version, &format!("{name}@{version}"))
}

/// packages/<link_name>, a link to the version's shallow clone in the machine's package cache (a folder package: to
/// its folder there): fetched once per machine, never again per checkout, export or test run. A link into the cache to
/// another version (the pin moved) or to a removed clone is replaced. A clean clone of the repository there (an
/// unpinned fetch) moves to packages/.replaced; anything else there (a local checkout, a clone with changes) stands in
/// for the pin, with a warning.
pub(super) fn fetch_tagged(
	packages: &Path,
	name: &str,
	package: &Registered,
	version: &Version,
	link_name: &str,
) -> Result<PathBuf, String> {
	pub(super) static LINKING: Mutex<()> = Mutex::new(()); // clone() takes FETCHING itself
	let _linking = LINKING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let cached = cached_package(name, version);
	let target = package.folder.as_ref().map_or(cached.clone(), |folder| cached.join(folder));
	let link = packages.join(link_name);
	let stale = |linked: PathBuf| linked.starts_with(package_cache()) && (linked != target || !target.exists());
	if std::fs::read_link(&link).is_ok_and(stale) {
		std::fs::remove_file(&link)
			.map_err(|failure| format!("package {name}: stale link {}: {failure}", link.display()))?;
	}
	if is_clean_clone(&link, &package.url) {
		set_aside(packages, name, version, &link)?;
	}
	if link.exists() {
		if std::fs::read_link(&link).map_or(true, |linked| linked != target) {
			warn_overridden(name, version, &link);
		}
		return Ok(link);
	}
	if !cached.exists() {
		let tags = package_version_tags(name, package)?;
		let tag = tags
			.iter()
			.find(|(_, tagged)| tagged == version)
			.map(|(tag, _)| tag.as_str())
			.ok_or(format!("package {name} has no tag of version {version} at {}", package.url))?;
		clone(name, &package.url, &cached, Some(tag))?;
	}
	link_package(packages, name, link_name, &target)
}

/// packages/<link_name> → target, unless something is there already
fn link_package(packages: &Path, name: &str, link_name: &str, target: &Path) -> Result<PathBuf, String> {
	let link = packages.join(link_name);
	if link.exists() {
		return Ok(link);
	}
	make_package_directory(name, packages)?;
	#[cfg(not(unix))]
	return Err(format!("package {name}: not linked to {}: no symbolic links on this platform", target.display()));
	#[cfg(unix)]
	match std::os::unix::fs::symlink(target, &link) {
		Err(failure) if !link.exists() => Err(format!("package {name}: not linked to {}: {failure}", target.display())),
		_ => Ok(link),
	}
}

/// A directory (not a link) holding a git clone of url without any change: what an unpinned fetch left there
pub(super) fn is_clean_clone(directory: &Path, url: &str) -> bool {
	is_real_directory(directory)
		&& directory.join(".git").exists()
		&& git_in(directory, &["remote", "get-url", "origin"]).as_deref() == Ok(url)
		&& git_in(directory, &["status", "--porcelain"]).is_ok_and(|changes| changes.is_empty())
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
	registered(name).ok_or(format!("unknown package: {name}"))?
}

pub(super) fn package_version_tags(name: &str, package: &Registered) -> Result<Vec<(String, Version)>, String> {
	let url = &package.url;
	let prefix = package.folder.as_ref().map(|folder| format!("{folder}{FOLDER_TAG_SEPARATOR}")).unwrap_or_default();
	version_tags(url, &prefix).map_err(|failure| format!("package {name}: no versions from {url}: {failure}"))
}

pub(super) fn make_package_directory(name: &str, directory: &Path) -> Result<(), String> {
	std::fs::create_dir_all(directory).map_err(|failure| format!("package {name}: {failure}"))
}

pub(super) fn package_cache() -> PathBuf {
	PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(PACKAGE_CACHE)
}

/// The tags of a repository that name versions after the prefix, with their versions
pub(super) fn version_tags(url: &str, prefix: &str) -> Result<Vec<(String, Version)>, String> {
	Ok(git_in(Path::new("."), &["ls-remote", "--tags", "--refs", url])?
		.lines()
		.filter_map(|line| line.split("refs/tags/").nth(1))
		.filter_map(|tag| Some((tag.to_string(), Version::parse(tag.strip_prefix(prefix)?)?)))
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
		let reference = match (&package.pinned, &package.folder) {
			(Some(version), Some(folder)) => format!("{folder}{FOLDER_TAG_SEPARATOR}{version}"),
			(Some(version), None) => format!("v{version}"),
			(None, _) => "HEAD".to_string(),
		};
		let repository = PathBuf::from(format!("{}/{reference}", raw_repository(&package.url)));
		Ok(package.folder.map_or(repository.clone(), |folder| repository.join(folder)))
	}
}

/// Where GitHub serves the raw files of a repository to any page
#[cfg(not(feature = "native"))]
fn raw_repository(url: &str) -> String {
	url.trim_end_matches(".git").replace("https://github.com/", "https://raw.githubusercontent.com/")
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

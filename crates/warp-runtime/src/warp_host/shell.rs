//! The words of lib/shell.warp (card unix-words) warp cannot write itself: the basic Unix commands on files and the
//! working directory, with warp results, as warp-rust's src/modules/shell_adapters.rs gives them. A path is the
//! program's: `~` the home folder, a relative one in the working directory (which `cd` changes, for the program and the
//! commands it runs).
use crate::nodes::{self, Node};
use std::path::{Path, PathBuf};

/// The members of the module file the shell adds (and file.list, file.glob of lib/file.warp)
pub fn file(member: &str, arguments: &[Node]) -> Option<Result<Node, String>> {
	Some(match (member, arguments) {
		("list", [folder]) => names_in(&path_of(folder)).map(texts),
		("glob", [pattern]) => globbed(&path_of(pattern)).map(texts),
		("mkdir", [folder]) => written(&path_of(folder), |folder| std::fs::create_dir_all(folder)),
		("copy", [from, to]) => {
			let (from, to) = (path_of(from), path_of(to));
			written(&into_folder(&from, &to), |to| std::fs::copy(&from, to).map(|_| ()))
		}
		("move", [from, to]) => {
			let (from, to) = (path_of(from), path_of(to));
			written(&into_folder(&from, &to), |to| std::fs::rename(&from, to))
		}
		("touch", [path]) => written(&path_of(path), |path| {
			std::fs::OpenOptions::new().append(true).create(true).open(path)?.set_modified(std::time::SystemTime::now())
		}),
		("remove", [path]) => trashed(&path_of(path)),
		("find", [folder, pattern]) => {
			let mut found = vec![];
			let matches = |name: &str| matches_name(&pattern.content(), name);
			found_below(Path::new(&path_of(folder)), &matches, &mut found).map(|_| texts(found))
		}
		("size", [path]) => size_of(Path::new(&path_of(path))).map(|bytes| Node::Int(bytes as i64)).map_err(said),
		("info", [path]) => info_of(Path::new(&path_of(path))),
		_ => return None,
	})
}

/// The members of the module os the shell adds
pub fn os(member: &str, arguments: &[Node]) -> Option<Result<Node, String>> {
	Some(match (member, arguments) {
		// `cd ~/projects`: the new working directory
		("cd", [folder]) => {
			let folder = path_of(folder);
			std::env::set_current_dir(&folder).map_err(|problem| format!("{folder}: {problem}")).and_then(|_| cwd())
		}
		("which", [name]) => Ok(program_on_path(&name.content()).map_or(Node::Empty, Node::text)),
		// `open ./photo.png`: the file in its app, as the system's open command does
		("open", [path]) => opened(&path_of(path)),
		_ => return None,
	})
}

/// The working directory
pub fn cwd() -> Result<Node, String> {
	std::env::current_dir().map(|folder| Node::text(folder.to_string_lossy())).map_err(said)
}

/// A path as the program wrote it: `~` and `~/…` in the home folder, as the shell expands them; a relative path stays
/// relative to the working directory, the process's own (os.cd changes it)
pub fn path_of(node: &Node) -> String {
	home_expanded(&node.content())
}

/// `~` and `~/…` with the home folder in place of `~`, other paths as they are
pub fn home_expanded(path: &str) -> String {
	match path.strip_prefix('~') {
		Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{}{rest}", home().display()),
		_ => path.to_string(),
	}
}

fn home() -> PathBuf {
	std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn said(problem: impl std::fmt::Display) -> String {
	problem.to_string()
}

fn texts(items: Vec<String>) -> Node {
	Node::List(nodes::SQUARE_LIST, items.into_iter().map(Node::text).collect())
}

/// The change to `path` made; ø, or the failure naming the path
fn written(path: &str, change: impl FnOnce(&str) -> std::io::Result<()>) -> Result<Node, String> {
	change(path).map(|_| Node::Empty).map_err(|problem| format!("{path}: {problem}"))
}

/// The names of a folder's entries, sorted
fn names_in(folder: &str) -> Result<Vec<String>, String> {
	let entries = std::fs::read_dir(folder).map_err(|problem| format!("{folder}: {problem}"))?;
	let mut names: Vec<String> =
		entries.filter_map(|entry| Some(entry.ok()?.file_name().to_string_lossy().into_owned())).collect();
	names.sort();
	Ok(names)
}

/// `glob("src/*.rs")`: the paths matching a pattern whose wildcards `*` and `?` are in its last part, sorted
fn globbed(pattern: &str) -> Result<Vec<String>, String> {
	let (folder, name, prefix) = match pattern.rsplit_once('/') {
		Some(("", name)) => ("/", name, "/".to_string()),
		Some((folder, name)) => (folder, name, format!("{folder}/")),
		None => (".", pattern, String::new()),
	};
	let names = names_in(folder)?.into_iter().filter(|entry| matches_name(name, entry));
	Ok(names.map(|entry| format!("{prefix}{entry}")).collect())
}

/// Does a file's name match `pattern` with the wildcards `*` and `?`; a hidden name only when the pattern starts with
/// `.`, as in the shell
fn matches_name(pattern: &str, name: &str) -> bool {
	fn wild(pattern: &[char], name: &[char]) -> bool {
		match (pattern.first(), name.first()) {
			(None, None) => true,
			(Some('*'), _) => wild(&pattern[1..], name) || (!name.is_empty() && wild(pattern, &name[1..])),
			(Some('?'), Some(_)) => wild(&pattern[1..], &name[1..]),
			(Some(want), Some(got)) if want == got => wild(&pattern[1..], &name[1..]),
			_ => false,
		}
	}
	let (pattern_chars, name_chars) = (pattern.chars().collect::<Vec<_>>(), name.chars().collect::<Vec<_>>());
	(pattern.starts_with('.') || !name.starts_with('.')) && wild(&pattern_chars, &name_chars)
}

/// The target of a copy or move: into a folder keeping the name, else the path itself
fn into_folder(from: &str, to: &str) -> String {
	let target = Path::new(to);
	match (target.is_dir(), Path::new(from).file_name()) {
		(true, Some(name)) => target.join(name).to_string_lossy().into_owned(),
		_ => to.to_string(),
	}
}

/// `rm "old.txt"` with guard rails (card unix-words, user decision): one file or an empty folder moved to the Trash,
/// never a pattern, the root, the home folder or the working directory and the folders above it
fn trashed(path: &str) -> Result<Node, String> {
	if path.contains(['*', '?']) {
		return Err(format!("rm takes one path, no pattern: for file in glob({path:?}) {{ rm file }}"));
	}
	let target = Path::new(path).canonicalize().map_err(|problem| format!("{path}: {problem}"))?;
	let working = std::env::current_dir().map_err(said)?;
	let home = home().canonicalize().unwrap_or_else(|_| home());
	if working.starts_with(&target) || target == home || target.parent().is_none() {
		return Err(format!(
			"rm never removes {}: the root, the home folder, the working directory or a folder above it",
			target.display()
		));
	}
	let entries = std::fs::read_dir(&target).map_or(0, Iterator::count);
	if target.is_dir() && entries > 0 {
		return Err(format!("rm removes a folder only when empty: {path} has {entries} entries"));
	}
	written(path, |_| move_to_trash(&target))
}

/// The Trash of macOS below the home folder; elsewhere the freedesktop.org one, with its info folder beside it
fn move_to_trash(target: &Path) -> std::io::Result<()> {
	let home = home();
	let (trash, info) = match cfg!(target_os = "macos") {
		true => (home.join(".Trash"), None),
		false => (home.join(".local/share/Trash/files"), Some(home.join(".local/share/Trash/info"))),
	};
	std::fs::create_dir_all(&trash)?;
	let place = free_place(&trash, target.file_name().unwrap_or_default().to_string_lossy().as_ref());
	if let Some(info) = info {
		std::fs::create_dir_all(&info)?;
		let name = place.file_name().unwrap_or_default().to_string_lossy();
		std::fs::write(info.join(format!("{name}.trashinfo")), format!("[Trash Info]\nPath={}\n", target.display()))?;
	}
	// another volume than the Trash's: copied there, then removed
	std::fs::rename(target, &place).or_else(|_| match target.is_dir() {
		true => std::fs::create_dir(&place).and_then(|_| std::fs::remove_dir(target)),
		false => std::fs::copy(target, &place).and_then(|_| std::fs::remove_file(target)),
	})
}

/// `name` in the Trash, or `name 2`, `name 3`, … when that is taken, as the Finder numbers them
fn free_place(trash: &Path, name: &str) -> PathBuf {
	let (stem, extension) = match name.rsplit_once('.') {
		Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
		_ => (name, String::new()),
	};
	(1..)
		.map(|number| match number {
			1 => trash.join(name),
			_ => trash.join(format!("{stem} {number}{extension}")),
		})
		.find(|place| !place.exists())
		.unwrap_or_default()
}

/// `find_files(~/dev, "*.warp")`: the paths below a folder whose name matches, sorted, hidden folders skipped
fn found_below(folder: &Path, matches: &impl Fn(&str) -> bool, found: &mut Vec<String>) -> Result<(), String> {
	let entries = std::fs::read_dir(folder).map_err(|problem| format!("{}: {problem}", folder.display()))?;
	let mut entries: Vec<PathBuf> = entries.filter_map(|entry| Some(entry.ok()?.path())).collect();
	entries.sort();
	for entry in entries {
		let name = entry.file_name().unwrap_or_default().to_string_lossy().into_owned();
		if matches(&name) {
			found.push(entry.to_string_lossy().into_owned());
		}
		if entry.is_dir() && !entry.is_symlink() && !name.starts_with('.') {
			found_below(&entry, matches, found)?;
		}
	}
	Ok(())
}

/// The bytes of a file, or of all files below a folder, as du counts them
fn size_of(path: &Path) -> std::io::Result<u64> {
	let metadata = std::fs::symlink_metadata(path)?;
	if !metadata.is_dir() {
		return Ok(metadata.len());
	}
	std::fs::read_dir(path)?.try_fold(0, |total, entry| Ok(total + size_of(&entry?.path())?))
}

/// `info ./a.txt` → {size: 20, modified: 1760536980000, kind: "file"}: modified in milliseconds as clock() counts
fn info_of(path: &Path) -> Result<Node, String> {
	let metadata = std::fs::metadata(path).map_err(|problem| format!("{}: {problem}", path.display()))?;
	let modified = metadata.modified().map_err(said)?.duration_since(std::time::UNIX_EPOCH).map_err(said)?;
	let kind = if metadata.is_dir() { "folder" } else { "file" };
	let size = size_of(path).map_err(said)?;
	Ok(Node::map(vec![
		("size", Node::Int(size as i64)),
		("modified", Node::Int(modified.as_millis() as i64)),
		("kind", Node::text(kind)),
	]))
}

/// The first file named `name` in a folder of the PATH, as the shell finds the program a command runs
fn program_on_path(name: &str) -> Option<String> {
	let folders = std::env::var_os("PATH")?;
	std::env::split_paths(&folders)
		.map(|folder| folder.join(name))
		.find(|path| path.is_file())
		.map(|path| path.to_string_lossy().into_owned())
}

/// `open ./photo.png`: the file in its app (open on macOS, xdg-open elsewhere); ø
fn opened(path: &str) -> Result<Node, String> {
	let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
	let output = std::process::Command::new(opener)
		.arg(path)
		.stdin(std::process::Stdio::null())
		.output()
		.map_err(|problem| format!("cannot run {opener}: {problem}"))?;
	match output.status.success() {
		true => Ok(Node::Empty),
		false => Err(format!(
			"{opener} {path} failed ({}): {}",
			output.status,
			String::from_utf8_lossy(&output.stderr).trim()
		)),
	}
}

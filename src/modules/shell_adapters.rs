//! The words of lib/shell.warp (card unix-words) that warp cannot write itself: the basic Unix commands on files and
//! the working directory, with warp results; a child of std_adapters for its helpers
use super::*;
use std::path::{Path, PathBuf};

/// The members of the module file the shell adds
pub(super) fn file(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("mkdir", [folder]) => written(&path_of(folder)?, |folder| std::fs::create_dir_all(folder)),
		("copy", [from, to]) => {
			let (from, to) = (path_of(from)?, path_of(to)?);
			written(&into_folder(&from, &to), |to| std::fs::copy(&from, to).map(|_| ()))
		}
		("move", [from, to]) => {
			let (from, to) = (path_of(from)?, path_of(to)?);
			written(&into_folder(&from, &to), |to| std::fs::rename(&from, to))
		}
		("touch", [path]) => written(&path_of(path)?, |path| {
			std::fs::OpenOptions::new().append(true).create(true).open(path)?.set_modified(std::time::SystemTime::now())
		}),
		("remove", [path]) => trashed(&path_of(path)?),
		("find", [folder, pattern]) => {
			let mut found = vec![];
			found_below(Path::new(&path_of(folder)?), &name_matcher(&text_of(pattern)?)?, &mut found)?;
			Ok(texts(found))
		}
		("size", [path]) => size_of(Path::new(&path_of(path)?)).map(|bytes| Node::int(bytes as i64)).map_err(said),
		("info", [path]) => info_of(Path::new(&path_of(path)?)),
		_ => no_such_word(arguments),
	}
}

/// The members of the module os the shell adds
pub(super) fn os(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		// `cd ~/projects`: the new working directory
		("cd", [folder]) => std::env::set_current_dir(path_of(folder)?).map_err(said).and_then(|_| working_folder()),
		("which", [name]) => Ok(program_on_path(&text_of(name)?).map_or(Node::Empty, Node::Text)),
		// `open ./photo.png`: the file in its app, as the system's open command does
		#[cfg(feature = "native")]
		("open", [path]) => executed(&[OPEN_COMMAND.into(), path_of(path)?]).map(|_| Node::Empty),
		_ => no_such_word(arguments),
	}
}

#[cfg(all(feature = "native", target_os = "macos"))]
const OPEN_COMMAND: &str = "open";
#[cfg(all(feature = "native", not(target_os = "macos")))]
const OPEN_COMMAND: &str = "xdg-open";
/// The Trash of macOS below the home folder; elsewhere the freedesktop.org one, with its info folder beside it
const MAC_TRASH: &str = ".Trash";
const XDG_TRASH: &str = ".local/share/Trash";

/// The change to `path` made, after the sandbox allowed writing there; ø, or the failure naming the path
fn written(path: &str, change: impl FnOnce(&str) -> std::io::Result<()>) -> Result<Node, String> {
	crate::host::sandbox::check_write(path)?;
	change(path).map(|_| Node::Empty).map_err(|problem| format!("{path}: {problem}"))
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
	if working.starts_with(&target) || target == crate::util::dirs_home() || target.parent().is_none() {
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

fn move_to_trash(target: &Path) -> std::io::Result<()> {
	let home = crate::util::dirs_home();
	let (trash, info) = match cfg!(target_os = "macos") {
		true => (home.join(MAC_TRASH), None),
		false => (home.join(XDG_TRASH).join("files"), Some(home.join(XDG_TRASH).join("info"))),
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
	let mut entries: Vec<PathBuf> =
		std::fs::read_dir(folder).map_err(said)?.filter_map(|entry| Some(entry.ok()?.path())).collect();
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
	let info = serde_json::json!({"size": size, "modified": modified.as_millis() as u64, "kind": kind});
	Ok(crate::modules::foreign::node_of(&info))
}

/// The first file named `name` in a folder of the PATH, as the shell finds the program a command runs
fn program_on_path(name: &str) -> Option<String> {
	let folders = std::env::var_os("PATH")?;
	std::env::split_paths(&folders)
		.map(|folder| folder.join(name))
		.find(|path| path.is_file())
		.map(|path| path.to_string_lossy().into_owned())
}

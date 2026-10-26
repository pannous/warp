//! The file being compiled and what lies beside it: relative paths, WebAssembly modules and the names `use` resolves
use super::*;

/// The setting naming the lib/ folder warp reads, whichever checkout built it
pub(crate) const LIBRARY_SETTING: &str = "WARP_LIB";
/// The folder of lib/ that marks a warp checkout's library: the warp-written compiler
pub const COMPILER_FOLDER: &str = "compiler";
/// An installed warp's lib/ and compilers when not next to the binary: <prefix>/share/warp for <prefix>/bin/warp
const INSTALLED_SHARE_FOLDER: &str = "../share/warp";
/// The compiler files whose presence marks an installed folder (src/compiler/standalone.rs): warp.wasm, or its old
/// name compiler.wasm (transition)
const INSTALLED_COMPILERS: [&str; 2] = ["warp.wasm", "compiler.wasm"];

/// `path` as written in the program: a relative path is next to the program's file, or in the working directory for
/// inline code
pub fn beside_program(path: &str) -> String {
	let folder = program_file().as_deref().map(folder_of);
	match folder {
		Some(folder) if Path::new(path).is_relative() => folder.join(path).to_string_lossy().into_owned(),
		_ => path.to_string(),
	}
}

/// beside_program when that file is there, else `path` as given: a file the program wrote into the working directory
/// (`play "song.mp3"`, card play-beside-program; host-files.js besideProgram in the browser)
pub fn beside_program_if_there(path: &str) -> String {
	let beside = beside_program(path);
	if Path::new(&beside).exists() {
		beside
	} else {
		path.to_string()
	}
}





/// The file being compiled, None for inline code
pub fn program_file() -> Option<PathBuf> {
	crate::compiler::compile_context::read(|context| context.program_file.clone())
}

/// Compile `body` as the program of `file`: `use folder`, `use package` and `use project` start from its folder
pub fn with_program_file<R>(file: &Path, body: impl FnOnce() -> R) -> R {
	crate::compiler::compile_context::scoped(|context| &mut context.program_file, Some(file.to_path_buf()), body)
}

/// `path` as written in the program, for the words that read and write files: `~` the home folder, a relative path in
/// the program's working folder
pub fn in_working_folder(path: &str) -> String {
	let path = crate::util::home_expanded(path);
	match working_folder() {
		Some(folder) if Path::new(&path).is_relative() => folder.join(path).to_string_lossy().into_owned(),
		_ => path,
	}
}

/// The folder the program's relative paths are in, None for the process's working directory
pub fn working_folder() -> Option<PathBuf> {
	crate::compiler::compile_context::read(|context| context.working_folder.clone())
}

/// Run `body` with `folder` as its working folder, the process's own working directory left as it is
pub fn with_working_folder<R>(folder: &Path, body: impl FnOnce() -> R) -> R {
	crate::compiler::compile_context::scoped(|context| &mut context.working_folder, Some(folder.to_path_buf()), body)
}

/// Where an installed warp keeps lib/ and its compilers (card installed-compilers): next to the binary (a release
/// tarball, main's ~/dev/bin), else <prefix>/share/warp beside the binary's bin/ (Homebrew, deb and rpm, the PyPI
/// wheel); the binary's own folder when neither holds them
pub fn installed_folder() -> Option<PathBuf> {
	let binary = std::env::current_exe().ok()?.canonicalize().ok()?;
	let folder = binary.parent()?.to_path_buf();
	let shared = folder.join(INSTALLED_SHARE_FOLDER);
	let holds_warp = |candidate: &PathBuf| {
		INSTALLED_COMPILERS.iter().any(|name| candidate.join(name).exists())
			|| candidate.join(STD_FOLDER).join(COMPILER_FOLDER).is_dir()
	};
	Some(if !holds_warp(&folder) && holds_warp(&shared) { shared } else { folder })
}

/// lib/ of the warp checkout at hand, so one warp build serves every worktree (card shared-warp-build): WARP_LIB, else
/// the nearest lib/ holding compiler/ above the program file or the working directory, else the installed one, else the
/// one warp was built from
pub fn library_folder() -> PathBuf {
	if let Some(folder) = crate::setting(LIBRARY_SETTING) {
		return PathBuf::from(folder);
	}
	let program_folder = program_file().and_then(|file| std::path::absolute(folder_of(&file)).ok());
	let starts = program_folder.into_iter().chain(std::env::current_dir().ok());
	let checkout_library = starts
		.flat_map(|start| start.ancestors().map(|folder| folder.join(STD_FOLDER)).collect::<Vec<_>>())
		.find(|library| library.join(COMPILER_FOLDER).is_dir());
	let installed_library = || {
		installed_folder()
			.map(|folder| folder.join(STD_FOLDER))
			.filter(|library| library.join(COMPILER_FOLDER).is_dir())
	};
	checkout_library
		.or_else(installed_library)
		.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(STD_FOLDER))
}

pub(crate) fn folder_of(file: &Path) -> PathBuf {
	match file.parent() {
		Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
		_ => PathBuf::from("."),
	}
}

/// The compile reads `what`, beyond the folders a kept program's key lists (src/run/program_cache.rs): its module is
/// not kept
pub fn reads_beyond_folders(what: &'static str) {
	#[cfg(feature = "native")]
	crate::run::program_cache::not_kept(what);
	#[cfg(not(feature = "native"))]
	let _ = what;
}

/// The compile looks for these module files, there or not: a kept program compiles again once one changes
pub fn looks_for(files: &[PathBuf]) {
	#[cfg(feature = "native")]
	for file in files {
		crate::run::program_cache::looks_for(file);
	}
	#[cfg(not(feature = "native"))]
	let _ = files;
}

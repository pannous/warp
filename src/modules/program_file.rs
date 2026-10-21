//! The file being compiled and what lies beside it: relative paths, WebAssembly modules and the names `use` resolves
use super::*;

/// The setting naming the lib/ folder warp reads, whichever checkout built it
pub(crate) const LIBRARY_SETTING: &str = "WARP_LIB";
/// The folder of lib/ that marks a warp checkout's library: the warp-written compiler
pub const COMPILER_FOLDER: &str = "compiler";

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

/// The WebAssembly file `use name` finds when no warp module and no standard module has that name: `use rust_demo` →
/// ./rust_demo.wasm
pub fn wasm_module_file(name: &str) -> Option<PathBuf> {
	if names_a_module(name) {
		return None;
	}
	program_loader().find_with(name, &crate::modules::wasm_modules::MODULE_EXTENSIONS)
}

/// Does `use name` name anything: a module (warp, standard, WebAssembly), a scope, a package or a C library
pub fn resolves(name: &str) -> bool {
	names_a_module(name)
		|| wasm_module_file(name).is_some()
		|| scope_word(name).is_some()
		|| package_repository(name).is_some()
		|| is_builtin_library(name)
		|| crate::ffi::is_ffi_library(name)
}

/// Is `name` a warp module beside the program or in the search directories, or a standard module
pub fn names_a_module(name: &str) -> bool {
	program_loader().find(name).is_some() || std_module(name).is_some()
}

pub(super) fn program_loader() -> Loader<'static> {
	Loader::new(&SEARCH_DIRECTORIES, program_file().as_deref().map(folder_of))
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

/// lib/ of the warp checkout at hand, so one warp build serves every worktree (card shared-warp-build): WARP_LIB, else
/// the nearest lib/ holding compiler/ above the program file or the working directory, else the one warp was built from
pub fn library_folder() -> PathBuf {
	if let Some(folder) = crate::setting(LIBRARY_SETTING) {
		return PathBuf::from(folder);
	}
	let program_folder = program_file().and_then(|file| std::path::absolute(folder_of(&file)).ok());
	let starts = program_folder.into_iter().chain(std::env::current_dir().ok());
	let checkout_library = starts
		.flat_map(|start| start.ancestors().map(|folder| folder.join(STD_FOLDER)).collect::<Vec<_>>())
		.find(|library| library.join(COMPILER_FOLDER).is_dir());
	checkout_library.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(STD_FOLDER))
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

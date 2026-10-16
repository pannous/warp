//! Compiled programs cached on disk (card cache-compiled-shader): `warp <file>` keeps the module its program compiles
//! to, so the next run of the same program skips the compile (1.2 s for samples/gpu_visualizer_warp.warp, whose paint
//! functions become a shader). The key covers everything the compile reads: warp itself (the binary's size and time),
//! the source, the program's path, the working directory, the WARP_ settings, `warp fast`, and the files of the folders where its
//! `use`s look (modules::SEARCH_DIRECTORIES below the program's folder and the working directory: names, sizes, times).
//! A compile that reads beyond them (packages, `use project`, a path, a C library) or says anything (a hint, a warning)
//! is not kept: its next run compiles and says it again. Kept next to the compiled machine code (module_cache.rs),
//! `WARP_MODULE_CACHE=off` turns both off.
use crate::pipeline::CompiledModule;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// The extension of a kept program in the module cache
pub const PROGRAM_EXTENSION: &str = "program";
/// The settings that change a compile are named so
const SETTINGS_PREFIX: &str = "WARP_";
/// The kept bytes: one byte of the imports the module needs, then the module
const NEEDS_HOST: u8 = 1;
const NEEDS_WASI: u8 = 2;
const NEEDS_FFI: u8 = 4;

/// Only the warp CLI running a program file keeps programs: tests and hosts compile what they are given
static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
	/// Why the compile running on this thread may not be kept, when it read beyond the key
	static NOT_KEPT: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The program file `warp` runs from now on keeps its compiled module (main.rs)
pub fn enable() {
	ENABLED.store(true, Ordering::Relaxed);
}

/// The compile running now read `what`, which the key does not cover: its module is not kept
pub fn not_kept(what: &'static str) {
	NOT_KEPT.with(|reason| reason.set(Some(what)));
}

/// The module kept for this program text, when warp keeps programs and has it
pub fn kept(code: &str) -> Option<CompiledModule> {
	let bytes = crate::run::module_cache::kept_bytes(&key(code)?, PROGRAM_EXTENSION)?;
	let (&needs, module) = bytes.split_first()?;
	Some(CompiledModule {
		bytes: module.to_vec(),
		needs_host: needs & NEEDS_HOST != 0,
		needs_wasi: needs & NEEDS_WASI != 0,
		needs_ffi: needs & NEEDS_FFI != 0,
	})
}

/// The compile of `code` by `compile`, kept when it read nothing beyond the key and said nothing
pub fn keeping<E>(code: &str, compile: impl FnOnce() -> Result<CompiledModule, E>) -> Result<CompiledModule, E> {
	NOT_KEPT.with(|reason| reason.set(None));
	let said_before = crate::errors::diagnostic::said();
	let compiled = compile()?;
	let quiet = crate::errors::diagnostic::said() == said_before;
	if let Some(key) = key(code).filter(|_| quiet && NOT_KEPT.with(Cell::get).is_none()) {
		let needs =
			[(compiled.needs_host, NEEDS_HOST), (compiled.needs_wasi, NEEDS_WASI), (compiled.needs_ffi, NEEDS_FFI)]
				.iter()
				.filter(|(needed, _)| *needed)
				.fold(0, |all, (_, bit)| all | bit);
		let bytes: Vec<u8> = std::iter::once(needs).chain(compiled.bytes.iter().copied()).collect();
		crate::run::module_cache::keep_bytes(&key, PROGRAM_EXTENSION, &bytes);
	}
	Ok(compiled)
}

/// The cache key of `code` as a program file run from here; none when warp keeps no programs
fn key(code: &str) -> Option<String> {
	if !ENABLED.load(Ordering::Relaxed) {
		return None;
	}
	let binary = std::env::current_exe().ok()?;
	let program = crate::modules::program_file()?.canonicalize().ok()?;
	let working = std::env::current_dir().ok()?;
	let mut settings: Vec<(String, String)> =
		std::env::vars().filter(|(name, _)| name.starts_with(SETTINGS_PREFIX)).collect();
	settings.sort();
	let mut hasher = Sha256::new();
	hasher.update(format!("{}\n{:?}\n", binary.display(), stamp(&binary)));
	let fast_math = crate::pipeline::fast_math();
	hasher.update(format!("{}\n{}\n{settings:?}\n{fast_math}\n", program.display(), working.display()));
	for folder in searched_folders(&program, &working) {
		hasher.update(format!("{}\n{:?}\n", folder.display(), listing(&folder)));
	}
	hasher.update(code);
	Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Whether a module found in `folder` lies in a folder the key lists
pub fn is_searched_folder(folder: &Path) -> bool {
	let (Some(program), Ok(working)) = (crate::modules::program_file(), std::env::current_dir()) else {
		return false;
	};
	let folder = resolved(folder);
	searched_folders(&program, &working).iter().any(|searched| resolved(searched) == folder)
}

/// The absolute path without links, also of a folder that does not exist (yet): a lookup there is keyed all the same
fn resolved(path: &Path) -> PathBuf {
	let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
	if let Ok(canonical) = absolute.canonicalize() {
		return canonical;
	}
	match (absolute.parent(), absolute.file_name()) {
		(Some(parent), Some(name)) => resolved(parent).join(name),
		_ => absolute,
	}
}

/// Where the program's `use`s look: each search directory below its folder and below the working directory
fn searched_folders(program: &Path, working: &Path) -> Vec<PathBuf> {
	let program_folder = program.parent().unwrap_or(Path::new("")).to_path_buf();
	[program_folder, working.to_path_buf()]
		.iter()
		.flat_map(|base| crate::modules::SEARCH_DIRECTORIES.iter().map(move |directory| base.join(directory)))
		.collect()
}

/// A folder's files with their size and time, sorted; empty when there is no such folder. Subfolders are left out:
/// their times change with whatever they hold (the cache itself may lie below), and a `use` naming a subfolder file is
/// not kept (modules::reads_beyond_folders)
fn listing(folder: &Path) -> Vec<(String, Option<(u64, std::time::SystemTime)>)> {
	let mut files: Vec<_> = std::fs::read_dir(folder)
		.into_iter()
		.flatten()
		.flatten()
		.filter(|entry| !entry.path().is_dir())
		.map(|entry| (entry.file_name().to_string_lossy().into_owned(), stamp(&entry.path())))
		.collect();
	files.sort();
	files
}

/// A file's size and modification time
fn stamp(path: &Path) -> Option<(u64, std::time::SystemTime)> {
	let metadata = std::fs::metadata(path).ok()?;
	Some((metadata.len(), metadata.modified().ok()?))
}

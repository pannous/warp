//! Compiled programs cached on disk (card cache-compiled-shader): `warp <file>` keeps the module its program compiles
//! to, so the next run of the same program skips the compile (1.2 s for samples/sound/gpu_visualizer_warp.warp, whose paint
//! functions become a shader). The key covers everything the compile reads: warp itself (the binary's size and time),
//! the source, the program's path, the working directory, the WARP_ settings, `warp fast`, and the files of the folders where its
//! `use`s look (modules::SEARCH_DIRECTORIES below the program's folder and the working directory: names, sizes, times
//! of the module files there, so a file the program writes beside it, `save "x.png"`, leaves the key as it was).
//! A module file looked for elsewhere (`use "../lib/x.warp"`, a subfolder's) is kept with its stamp: the program compiles
//! again once it changes, appears or goes. A compile that reads beyond them (packages, `use project`, a C library) or
//! asks (an assumption) is not kept; the hints and warnings a kept compile said are said again by each run from its
//! module. Kept next to the compiled machine code (module_cache.rs), `WARP_MODULE_CACHE=off` turns both off.
use crate::errors::diagnostic::{recording_said, say, Diagnostic};
use crate::pipeline::CompiledModule;
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The extension of a kept program in the module cache
pub const PROGRAM_EXTENSION: &str = "program";
/// The settings that change a compile are named so
const SETTINGS_PREFIX: &str = "WARP_";
/// The kept bytes: one byte of the imports the module needs, then the Compiled (postcard), then the module
const NEEDS_HOST: u8 = 1;
const NEEDS_WASI: u8 = 2;
const NEEDS_FFI: u8 = 4;

/// The running warp binary's path and stamp, taken once at start: only the warp CLI running a program file keeps
/// programs (tests and hosts compile what they are given). A rebuild replacing the binary while this run compiles must
/// not key the old compiler's module as the new one's: its first runs printed the old output (card probes-printed)
static BINARY: OnceLock<String> = OnceLock::new();

thread_local! {
	/// Why the compile running on this thread may not be kept, when it read beyond the key
	static NOT_KEPT: Cell<Option<&'static str>> = const { Cell::new(None) };
	/// The module files the compile kept on this thread (keeping) looked for, with their stamps then
	static LOOKED_FOR: RefCell<Option<BTreeMap<PathBuf, Option<Stamp>>>> = const { RefCell::new(None) };
}

/// A file's size and modification time
pub(crate) type Stamp = (u64, std::time::SystemTime);

/// What a kept module's compile depended on beyond its key, and what it said
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Compiled {
	looked_for: Vec<(PathBuf, Option<Stamp>)>,
	said: Vec<Diagnostic>,
}

impl Compiled {
	/// Whether every file it looked for is as it was then
	fn is_current(&self) -> bool {
		self.looked_for.iter().all(|(path, then)| stamp(path) == *then)
	}
}

/// The program file `warp` runs from now on keeps its compiled module (main.rs)
pub fn enable() {
	if let Ok(binary) = std::env::current_exe() {
		BINARY.get_or_init(|| format!("{}\n{:?}\n", binary.display(), stamp(&binary)));
	}
}

/// The compile running now read `what`, which the key does not cover: its module is not kept
pub fn not_kept(what: &'static str) {
	NOT_KEPT.with(|reason| reason.set(Some(what)));
}

/// The compile looks for the module file `path`, there or not: a kept program compiles again once it changes
pub fn looks_for(path: &Path) {
	LOOKED_FOR.with(|files| {
		if let Some(files) = files.borrow_mut().as_mut() {
			files.entry(path.to_path_buf()).or_insert_with(|| stamp(path));
		}
	});
}

/// The module kept for this program text, when warp keeps programs and has it
pub fn kept(code: &str) -> Option<CompiledModule> {
	kept_under(&key(code)?)
}

/// The module kept under `key` while the files its compile looked for are unchanged, saying again what that compile
/// said; for a compile whose inputs its caller keys itself too (self_hosted.rs stage programs)
pub fn kept_under(key: &str) -> Option<CompiledModule> {
	let bytes = crate::run::module_cache::kept_bytes(key, PROGRAM_EXTENSION)?;
	let (&needs, rest) = bytes.split_first()?;
	let (compiled, module) = postcard::take_from_bytes::<Compiled>(rest).ok()?;
	if !compiled.is_current() {
		return None;
	}
	for diagnostic in compiled.said {
		say(diagnostic);
	}
	Some(CompiledModule {
		bytes: module.to_vec(),
		needs_host: needs & NEEDS_HOST != 0,
		needs_wasi: needs & NEEDS_WASI != 0,
		needs_ffi: needs & NEEDS_FFI != 0,
	})
}

/// The compile of `code` by `compile`, kept when it read nothing beyond the key and the files it looked for, and said
/// nothing but what say() says again
pub fn keeping<E>(code: &str, compile: impl FnOnce() -> Result<CompiledModule, E>) -> Result<CompiledModule, E> {
	NOT_KEPT.with(|reason| reason.set(None));
	LOOKED_FOR.with(|files| files.replace(Some(BTreeMap::new())));
	let key = key(code); // before the compile: what it read is keyed as it was then
	let (compiled, said, only_said) = recording_said(compile);
	let looked_for = LOOKED_FOR.with(|files| files.take()).unwrap_or_default().into_iter().collect();
	let module = compiled?;
	if let Some(key) = key.filter(|_| only_said && NOT_KEPT.with(Cell::get).is_none()) {
		keep_with(&key, &module, &Compiled { looked_for, said });
	}
	Ok(module)
}

/// Keep `compiled` under `key`
pub fn keep_under(key: &str, compiled: &CompiledModule) {
	keep_with(key, compiled, &Compiled::default());
}

fn keep_with(key: &str, module: &CompiledModule, compiled: &Compiled) {
	let needs = [(module.needs_host, NEEDS_HOST), (module.needs_wasi, NEEDS_WASI), (module.needs_ffi, NEEDS_FFI)]
		.iter()
		.filter(|(needed, _)| *needed)
		.fold(0, |all, (_, bit)| all | bit);
	let Ok(dependencies) = postcard::to_allocvec(compiled) else {
		return;
	};
	let bytes: Vec<u8> = std::iter::once(needs).chain(dependencies).chain(module.bytes.iter().copied()).collect();
	crate::run::module_cache::keep_bytes(key, PROGRAM_EXTENSION, &bytes);
}

/// The WARP_ settings, which change a compile, sorted
pub(crate) fn warp_settings() -> Vec<(String, String)> {
	let mut settings: Vec<(String, String)> =
		std::env::vars().filter(|(name, _)| name.starts_with(SETTINGS_PREFIX)).collect();
	settings.sort();
	settings
}

/// The cache key of `code` as a program file run from here; none when warp keeps no programs
fn key(code: &str) -> Option<String> {
	let binary = BINARY.get()?;
	let program = crate::modules::program_file()?.canonicalize().ok()?;
	let working = std::env::current_dir().ok()?;
	let settings = warp_settings();
	let mut hasher = Sha256::new();
	hasher.update(binary);
	let fast_math = crate::pipeline::fast_math();
	hasher.update(format!("{}\n{}\n{settings:?}\n{fast_math}\n", program.display(), working.display()));
	for folder in searched_folders(&program, &working) {
		hasher.update(format!("{}\n{:?}\n", folder.display(), listing(&folder)));
	}
	hasher.update(crate::compiler::self_hosted::compiler_key().unwrap_or_default());
	hasher.update(code);
	Some(crate::extensions::hex_text(&hasher.finalize()))
}

/// Where the program's `use`s look: each search directory below its folder and below the working directory
fn searched_folders(program: &Path, working: &Path) -> Vec<PathBuf> {
	let program_folder = program.parent().unwrap_or(Path::new("")).to_path_buf();
	[program_folder, working.to_path_buf()]
		.iter()
		.flat_map(|base| crate::modules::SEARCH_DIRECTORIES.iter().map(move |directory| base.join(directory)))
		.collect()
}

/// A folder's module files (.warp, .wasp, .wasm, .wat: what a `use` loads) with their size and time, sorted; empty when
/// there is no such folder. Subfolders are left out: their times change with whatever they hold (the cache itself may lie
/// below); a `use` naming a subfolder file keeps that file's stamp (looks_for)
fn listing(folder: &Path) -> Vec<(String, Option<Stamp>)> {
	let modules = crate::modules::MODULE_EXTENSIONS.iter().chain(&crate::modules::wasm_modules::MODULE_EXTENSIONS);
	let extensions: Vec<&str> = modules.copied().collect();
	let is_module =
		|path: &Path| path.extension().is_some_and(|extension| extensions.iter().any(|known| extension == *known));
	let mut files: Vec<_> = std::fs::read_dir(folder)
		.into_iter()
		.flatten()
		.flatten()
		.filter(|entry| !entry.path().is_dir() && is_module(&entry.path()))
		.map(|entry| (entry.file_name().to_string_lossy().into_owned(), stamp(&entry.path())))
		.collect();
	files.sort();
	files
}

/// A file's size and modification time
pub(crate) fn stamp(path: &Path) -> Option<Stamp> {
	let metadata = std::fs::metadata(path).ok()?;
	Some((metadata.len(), metadata.modified().ok()?))
}

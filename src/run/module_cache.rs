//! Compiled modules cached on disk (notes/aot.md): wasmtime's ahead-of-time output (.cwasm) of every module a run
//! compiles, keyed by the SHA-256 of the module bytes and the engine's compatibility hash (wasmtime version, target,
//! every setting that changes the machine code). A cached module is loaded instead of compiled again.
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, Once};
use std::time::SystemTime;
use wasmtime::component::Component;
use wasmtime::{Engine, Module, Precompiled, Result};

/// Environment variable naming the cache directory; `off` compiles every module without the cache
pub const CACHE_VARIABLE: &str = "WARP_MODULE_CACHE";
const CACHE_OFF: &str = "off";
/// Below the home directory, next to the package cache (modules.rs)
const DEFAULT_CACHE: &str = ".cache/warp/modules";
/// wasmtime's ahead-of-time output: the cache's files, and what `warp app.cwasm` runs as it is
pub const COMPILED_EXTENSION: &str = "cwasm";
/// A cache grown past this size drops its least recently used modules down to half of it (a test suite run writes
/// a few thousand modules of 100–200 KB)
const MAX_CACHE_BYTES: u64 = 1 << 30;

/// Modules this process holds compiled, by cache key; past this many it starts over (each holds 100–200 KB of code)
const MAX_MODULES_IN_MEMORY: usize = 512;

static PRUNE_ONCE: Once = Once::new();
/// The modules of the shared gc_engine compiled or loaded in this process: running one again is a lookup
static IN_MEMORY: LazyLock<Mutex<HashMap<String, Module>>> = LazyLock::new(Mutex::default);

/// What the cache holds: wasmtime's compiled code of a core module or of a component
pub trait CompiledCode: Sized {
	fn compile(engine: &Engine, bytes: &[u8]) -> Result<Self>;
	/// # Safety
	/// `bytes` must be wasmtime's own serialized output (see cached_code)
	unsafe fn deserialize(engine: &Engine, bytes: &[u8]) -> Result<Self>;
	fn serialize(&self) -> Result<Vec<u8>>;
}

impl CompiledCode for Module {
	fn compile(engine: &Engine, bytes: &[u8]) -> Result<Self> {
		Module::new(engine, bytes)
	}
	unsafe fn deserialize(engine: &Engine, bytes: &[u8]) -> Result<Self> {
		unsafe { Module::deserialize(engine, bytes) }
	}
	fn serialize(&self) -> Result<Vec<u8>> {
		Module::serialize(self)
	}
}

impl CompiledCode for Component {
	fn compile(engine: &Engine, bytes: &[u8]) -> Result<Self> {
		Component::new(engine, bytes)
	}
	unsafe fn deserialize(engine: &Engine, bytes: &[u8]) -> Result<Self> {
		unsafe { Component::deserialize(engine, bytes) }
	}
	fn serialize(&self) -> Result<Vec<u8>> {
		Component::serialize(self)
	}
}

/// Where a module came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
	Compiled,
	Cached,
}

/// `bytes` (a binary module or WAT text) compiled for `engine`: from this process's modules or the cache when they
/// hold it, else compiled and added to both
pub fn compiled_module(engine: &Engine, bytes: &[u8]) -> Result<Module> {
	crate::compiler::phase_times::timed("wasmtime compile", || compiled_module_untimed(engine, bytes))
}

fn compiled_module_untimed(engine: &Engine, bytes: &[u8]) -> Result<Module> {
	if is_precompiled(bytes) {
		// SAFETY: compiled code runs as it is, so a .cwasm file is trusted like a native executable: the user runs it
		return unsafe { Module::deserialize(engine, bytes) };
	}
	if let Some(module) = shipped_module(engine, bytes) {
		return module;
	}
	let Some(directory) = cache_directory() else {
		return Module::new(engine, bytes);
	};
	let key = cache_key(engine, bytes);
	if let Some(module) = remembered(engine, &key) {
		return Ok(module);
	}
	let (module, _) = load_or_compile(&directory, &key, engine, bytes)?;
	remember(key, &module);
	Ok(module)
}

/// A component (`use lib.wasm`) compiled for `engine`, from the cache when it holds it: compiling uniscript's
/// component takes most of a second (card takes-three)
pub fn compiled_component(engine: &Engine, bytes: &[u8]) -> Result<Component> {
	match cache_directory() {
		Some(directory) => Ok(compiled_in(&directory, engine, bytes)?.0),
		None => Component::new(engine, bytes),
	}
}

/// A module or component compiled for `engine` with the cache in `directory`, and where it came from
fn compiled_in<Code: CompiledCode>(directory: &Path, engine: &Engine, bytes: &[u8]) -> Result<(Code, Origin)> {
	load_or_compile(directory, &cache_key(engine, bytes), engine, bytes)
}

/// The module this process compiled for the same engine (a Module belongs to the Engine that compiled it)
fn remembered(engine: &Engine, key: &str) -> Option<Module> {
	let modules = IN_MEMORY.lock().ok()?;
	modules.get(key).filter(|module| Engine::same(module.engine(), engine)).cloned()
}

fn remember(key: String, module: &Module) {
	if let Ok(mut modules) = IN_MEMORY.lock() {
		if modules.len() >= MAX_MODULES_IN_MEMORY {
			modules.clear();
		}
		modules.insert(key, module.clone());
	}
}

fn load_or_compile<Code: CompiledCode>(
	directory: &Path,
	key: &str,
	engine: &Engine,
	bytes: &[u8],
) -> Result<(Code, Origin)> {
	let path = directory.join(format!("{key}.{COMPILED_EXTENSION}"));
	if let Some(module) = cached_code(engine, &path) {
		return Ok((module, Origin::Cached));
	}
	let module = Code::compile(engine, bytes)?;
	if let Err(failure) = store_code(directory, &path, &module) {
		eprintln!("warning: module cache {}: {failure}", directory.display());
	}
	Ok((module, Origin::Compiled))
}

/// Whether `bytes` are wasmtime's compiled output (`warp compile --aot`, a .cwasm file) instead of a module
pub fn is_precompiled(bytes: &[u8]) -> bool {
	Engine::detect_precompiled(bytes) == Some(Precompiled::Module)
}

/// Whether machine code (`warp compile --aot`) counts fuel, and whether it starts tasks: code compiled for the engine
/// counting fuel does not load into the one checking the time
pub fn precompiled_engine(bytes: &[u8]) -> (bool, bool) {
	// SAFETY: as in compiled_module
	let loaded = |fuel| unsafe { Module::deserialize(&crate::util::shared_engine(fuel), bytes) }.ok();
	match loaded(false) {
		Some(module) => (false, crate::host::tasks::imports_tasks(&module)),
		None => (true, loaded(true).is_some_and(|module| crate::host::tasks::imports_tasks(&module))),
	}
}

/// The cache directory: `WARP_MODULE_CACHE`, else ~/.cache/warp/modules; none when it is `off`
fn cache_directory() -> Option<PathBuf> {
	match std::env::var(CACHE_VARIABLE) {
		Ok(setting) if setting == CACHE_OFF => None,
		Ok(setting) if !setting.is_empty() => Some(PathBuf::from(setting)),
		_ => std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(DEFAULT_CACHE)),
	}
}

fn cache_key(engine: &Engine, bytes: &[u8]) -> String {
	let mut engine_hasher = std::collections::hash_map::DefaultHasher::new();
	engine.precompile_compatibility_hash().hash(&mut engine_hasher);
	crate::extensions::hex_text(
		&Sha256::new().chain_update(engine_hasher.finish().to_le_bytes()).chain_update(bytes).finalize(),
	)
}

/// The module at `path`, when there is one wasmtime accepts for `engine`; a hit counts as a use for pruning
fn cached_code<Code: CompiledCode>(engine: &Engine, path: &Path) -> Option<Code> {
	if !path.exists() {
		return None;
	}
	// Read, not deserialize_file: a mapped file stays open as long as its Module, and IN_MEMORY keeps up to 512, past
	// macOS's default limit of 256 open files (EMFILE in tests, card targeted-tests)
	let loaded = fs::read(path).map_err(wasmtime::Error::from).and_then(|bytes| {
		// SAFETY: deserialize trusts the file to be wasmtime's own output: only this module writes the cache directory,
		// atomically (store_code), and wasmtime still rejects a file of another version or engine configuration
		unsafe { Code::deserialize(engine, &bytes) }
	});
	match loaded {
		Ok(module) => {
			touch(path);
			Some(module)
		}
		Err(failure) => {
			eprintln!("warning: unusable cached module {} is compiled again: {failure}", path.display());
			None
		}
	}
}

fn store_code(directory: &Path, path: &Path, module: &impl CompiledCode) -> Result<()> {
	Ok(store_file(directory, path, &module.serialize()?)?)
}

/// Mark the cached file as just used, for pruning
fn touch(path: &Path) {
	let _ = fs::File::options().write(true).open(path).and_then(|file| file.set_modified(SystemTime::now()));
}

/// Write the bytes to a private file and rename it into place, so parallel runs never read half a file
fn store_file(directory: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
	fs::create_dir_all(directory)?;
	PRUNE_ONCE.call_once(|| prune(directory));
	let staging = path.with_extension(format!("{}-{:?}.tmp", std::process::id(), std::thread::current().id()));
	fs::write(&staging, bytes)?;
	fs::rename(&staging, path)
}

/// The bytes kept under `key` with `extension` (src/run/program_cache.rs), a hit counting as a use for pruning; none
/// without them or with the cache off
pub fn kept_bytes(key: &str, extension: &str) -> Option<Vec<u8>> {
	let name = format!("{key}.{extension}");
	let cached = cache_directory().map(|directory| directory.join(&name)).and_then(|path| {
		let bytes = fs::read(&path).ok()?;
		touch(&path);
		Some(bytes)
	});
	let (shipped, writing) = SHIPPED_KEPT.with(|kept| kept.borrow().clone());
	let Some(folder) = shipped else { return cached };
	match cached {
		Some(bytes) => {
			if writing {
				ship_kept(&folder, &name, &bytes);
			}
			Some(bytes)
		}
		None => fs::read(folder.join(&name)).ok(),
	}
}

/// Keep the bytes under `key` with `extension` in the cache, unless it is off
pub fn keep_bytes(key: &str, extension: &str, bytes: &[u8]) {
	if let (Some(folder), true) = SHIPPED_KEPT.with(|kept| kept.borrow().clone()) {
		ship_kept(&folder, &format!("{key}.{extension}"), bytes);
	}
	let Some(directory) = cache_directory() else { return };
	if let Err(failure) = store_file(&directory, &directory.join(format!("{key}.{extension}")), bytes) {
		eprintln!("warning: module cache {}: {failure}", directory.display());
	}
}

/// Drop the least recently used modules of a cache grown past MAX_CACHE_BYTES until it is half that size
fn prune(directory: &Path) {
	let Ok(entries) = fs::read_dir(directory) else { return };
	let mut modules: Vec<(SystemTime, u64, PathBuf)> = entries
		.filter_map(Result::ok)
		.filter_map(|entry| entry.metadata().ok().map(|metadata| (metadata, entry.path())))
		.filter(|(metadata, _)| metadata.is_file())
		.map(|(metadata, path)| (metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), metadata.len(), path))
		.collect();
	let mut total: u64 = modules.iter().map(|(_, size, _)| size).sum();
	if total <= MAX_CACHE_BYTES {
		return;
	}
	modules.sort();
	for (_, size, path) in modules {
		if total <= MAX_CACHE_BYTES / 2 {
			break;
		}
		if fs::remove_file(&path).is_ok() {
			total -= size;
		}
	}
}

/// A compiler's machine code ships beside its build file as `<file>.cwasm` (card precompiled-compilers; user rule: a
/// compiled compiler is never compiled to machine code at run time): `warp rebuild` writes it with the engine runs use,
/// and a run loads it as it is. The file is wasmtime's precompiled module followed by a stamp naming the module bytes
/// and the engine it is for, so a .cwasm of another build file or engine is never loaded
pub const SHIPPED_EXTENSION: &str = "cwasm";
const STAMP_MARK: &[u8; 16] = b"warp shipped 1\n\0";
/// The stamp: the module bytes' SHA-256, the engine's compatibility hash, STAMP_MARK
const STAMP_LENGTH: usize = 32 + 8 + STAMP_MARK.len();

/// The build files whose machine code ships beside them, by their bytes' SHA-256 (shipped_with)
static SHIPPED: LazyLock<Mutex<HashMap<Vec<u8>, PathBuf>>> = LazyLock::new(Mutex::default);

/// The .cwasm beside a build file
pub fn shipped_path(build_file: &Path) -> PathBuf {
	let mut name = build_file.as_os_str().to_owned();
	name.push(format!(".{SHIPPED_EXTENSION}"));
	PathBuf::from(name)
}

/// `bytes`, read from the build file at `build_file`, run from the machine code shipped beside it (compiled_module)
pub fn shipped_with(build_file: &Path, bytes: &[u8]) {
	if let Ok(mut shipped) = SHIPPED.lock() {
		shipped.insert(Sha256::digest(bytes).to_vec(), build_file.to_path_buf());
	}
}

fn stamp(engine: &Engine, bytes: &[u8]) -> Vec<u8> {
	let mut engine_hasher = std::collections::hash_map::DefaultHasher::new();
	engine.precompile_compatibility_hash().hash(&mut engine_hasher);
	let mut stamp = Sha256::digest(bytes).to_vec();
	stamp.extend(engine_hasher.finish().to_le_bytes());
	stamp.extend(STAMP_MARK);
	stamp
}

/// The machine code of `bytes` for `engine`, precompiled (Engine::precompile_module) and stamped, written beside the
/// build file: what `warp rebuild` and the seed refresh ship
pub fn write_shipped(engine: &Engine, build_file: &Path, bytes: &[u8]) -> Result<PathBuf> {
	let mut code = engine.precompile_module(bytes)?;
	code.extend(stamp(engine, bytes));
	let path = shipped_path(build_file);
	store_shipped(&path, &code)?;
	Ok(path)
}

fn store_shipped(path: &Path, code: &[u8]) -> std::io::Result<()> {
	let staging = path.with_extension(format!("{SHIPPED_EXTENSION}.{}.tmp", std::process::id()));
	fs::write(&staging, code)?;
	fs::rename(&staging, path)
}

/// Whether the .cwasm beside `build_file` is the machine code of `bytes` for `engine`
pub fn shipped_current(engine: &Engine, build_file: &Path, bytes: &[u8]) -> bool {
	shipped_mismatch(engine, &shipped_path(build_file), bytes).is_none()
}

/// Why the .cwasm at `path` is not the machine code of `bytes` for `engine`, None when it is
fn shipped_mismatch(engine: &Engine, path: &Path, bytes: &[u8]) -> Option<&'static str> {
	use std::io::{Read, Seek, SeekFrom};
	let Ok(mut file) = fs::File::open(path) else { return Some("missing") };
	let mut found = vec![0; STAMP_LENGTH];
	let read = file.seek(SeekFrom::End(-(STAMP_LENGTH as i64))).and_then(|_| file.read_exact(&mut found));
	let wanted = stamp(engine, bytes);
	match () {
		_ if read.is_err() || !found.ends_with(STAMP_MARK) => Some("unstamped"),
		_ if found[..32] != wanted[..32] => Some("stale (of other module bytes)"),
		_ if found[32..40] != wanted[32..40] => Some("for another engine"),
		_ => None,
	}
}

/// A shipped build file's module from the machine code beside it; None for any other module. A missing or stale .cwasm
/// is compiled once, said in one line, and written back
fn shipped_module(engine: &Engine, bytes: &[u8]) -> Option<Result<Module>> {
	let build_file = SHIPPED.lock().ok()?.get(Sha256::digest(bytes).as_slice()).cloned()?;
	let path = shipped_path(&build_file);
	let key = format!("shipped {}", cache_key(engine, bytes));
	if let Some(module) = remembered(engine, &key) {
		return Some(Ok(module));
	}
	if shipped_mismatch(engine, &path, bytes).is_none() {
		// SAFETY: a .cwasm stamped with these module bytes and this engine is wasmtime's own output, written by
		// write_shipped or shipped_module; wasmtime checks its version, settings and CPU features again
		match unsafe { Module::deserialize_file(engine, &path) } {
			Ok(module) => {
				remember(key, &module);
				return Some(Ok(module));
			}
			Err(failure) => eprintln!("warp: {} unusable ({failure}): compiling once", path.display()),
		}
	} else {
		let why = shipped_mismatch(engine, &path, bytes).unwrap_or("unusable");
		eprintln!(
			"warp: {} {why}: compiling {} to machine code once (~30 s), written back",
			path.display(),
			build_file.display()
		);
	}
	let module = match Module::new(engine, bytes) {
		Ok(module) => module,
		Err(failure) => return Some(Err(failure)),
	};
	let written =
		module.serialize().map_err(|failure| std::io::Error::other(failure.to_string())).and_then(|mut code| {
			code.extend(stamp(engine, bytes));
			store_shipped(&path, &code)
		});
	if let Err(failure) = written {
		eprintln!("warp: cannot write {} back: {failure}", path.display());
	}
	remember(key, &module);
	Some(Ok(module))
}

thread_local! {
	/// The kept values shipped beside the compiler running (`<file>.kept/`), and whether this run writes them there
	static SHIPPED_KEPT: std::cell::RefCell<(Option<PathBuf>, bool)> = const { std::cell::RefCell::new((None, false)) };
}

/// The folder of kept values shipped beside the build file `bytes` were read from (shipped_with), if any
pub fn shipped_kept_folder(bytes: &[u8]) -> Option<PathBuf> {
	let build_file = SHIPPED.lock().ok()?.get(Sha256::digest(bytes).as_slice()).cloned()?;
	Some(shipped_kept_folder_of(&build_file))
}

/// The folder of kept values beside a build file: `<file>.kept/`
pub fn shipped_kept_folder_of(build_file: &Path) -> PathBuf {
	let mut name = build_file.as_os_str().to_owned();
	name.push(".kept");
	PathBuf::from(name)
}

/// `run` with the values a compiler keeps (os.keep: its lowered runtime words, the parses of the rules modules) read
/// from `folder` when the cache lacks them, a cold cache included, and written there too when `writing` (`warp
/// rebuild` warming a new build): a compiler's first run in a fresh cache need not compute them (card
/// precompiled-compilers)
pub fn with_shipped_kept<R>(folder: Option<PathBuf>, writing: bool, run: impl FnOnce() -> R) -> R {
	let before = SHIPPED_KEPT.with(|kept| kept.replace((folder, writing)));
	let result = run();
	SHIPPED_KEPT.with(|kept| kept.replace(before));
	result
}

fn ship_kept(folder: &Path, name: &str, bytes: &[u8]) {
	let path = folder.join(name);
	let written = fs::create_dir_all(folder).and_then(|_| store_shipped(&path, bytes));
	if let Err(failure) = written {
		eprintln!("warning: cannot ship kept value {}: {failure}", path.display());
	}
}

/// Whether this thread warms a new build: its compiler runs write their kept values beside it
pub fn warming() -> bool {
	WARMING.with(std::cell::Cell::get)
}

/// `run` as a new build's warming run (warming)
pub fn warming_with<R>(run: impl FnOnce() -> R) -> R {
	let before = WARMING.with(|warming| warming.replace(true));
	let result = run();
	WARMING.with(|warming| warming.replace(before));
	result
}

thread_local! {
	static WARMING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

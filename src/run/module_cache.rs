//! Compiled modules cached on disk (notes/aot.md): wasmtime's ahead-of-time output (.cwasm) of every module a run
//! compiles, keyed by the SHA-256 of the module bytes and the engine's compatibility hash (wasmtime version, target,
//! every setting that changes the machine code). A cached module is mapped into memory instead of compiled again.
use sha2::{Digest, Sha256};
use std::fs;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, Once};
use std::time::SystemTime;
use wasmtime::{Engine, Module, Precompiled, Result};

/// Environment variable naming the cache directory; `off` compiles every module without the cache
pub const CACHE_VARIABLE: &str = "WARP_MODULE_CACHE";
const CACHE_OFF: &str = "off";
/// Below the home directory, next to the package cache (modules.rs)
const DEFAULT_CACHE: &str = ".cache/warp/modules";
const COMPILED_EXTENSION: &str = "cwasm";
/// A cache grown past this size drops its least recently used modules down to half of it (a test suite run writes
/// a few thousand modules of 100–200 KB)
const MAX_CACHE_BYTES: u64 = 1 << 30;

/// Modules this process holds compiled, by cache key; past this many it starts over (each holds 100–200 KB of code)
const MAX_MODULES_IN_MEMORY: usize = 512;

static PRUNE_ONCE: Once = Once::new();
/// The modules of the shared gc_engine compiled or loaded in this process: running one again is a lookup
static IN_MEMORY: LazyLock<Mutex<HashMap<String, Module>>> = LazyLock::new(Mutex::default);

/// Where a module came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
	Compiled,
	Cached,
}

/// `bytes` (a binary module or WAT text) compiled for `engine`: from this process's modules or the cache when they
/// hold it, else compiled and added to both
pub fn compiled_module(engine: &Engine, bytes: &[u8]) -> Result<Module> {
	if is_precompiled(bytes) {
		// SAFETY: compiled code runs as it is, so a .cwasm file is trusted like a native executable: the user runs it
		return unsafe { Module::deserialize(engine, bytes) };
	}
	let Some(directory) = cache_directory() else { return Module::new(engine, bytes) };
	let key = cache_key(engine, bytes);
	if let Some(module) = remembered(engine, &key) {
		return Ok(module);
	}
	let (module, _) = load_or_compile(&directory, &key, engine, bytes)?;
	remember(key, &module);
	Ok(module)
}

/// compiled_module with the cache in `directory` and without this process's modules, and where the module came from
pub fn compiled_module_in(directory: &Path, engine: &Engine, bytes: &[u8]) -> Result<(Module, Origin)> {
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

fn load_or_compile(directory: &Path, key: &str, engine: &Engine, bytes: &[u8]) -> Result<(Module, Origin)> {
	let path = directory.join(format!("{key}.{COMPILED_EXTENSION}"));
	if let Some(module) = cached_module(engine, &path) {
		return Ok((module, Origin::Cached));
	}
	let module = Module::new(engine, bytes)?;
	if let Err(failure) = store_module(directory, &path, &module) {
		eprintln!("warning: module cache {}: {failure}", directory.display());
	}
	Ok((module, Origin::Compiled))
}

/// Whether `bytes` are wasmtime's compiled output (`warp compile --aot`, a .cwasm file) instead of a module
pub fn is_precompiled(bytes: &[u8]) -> bool {
	Engine::detect_precompiled(bytes) == Some(Precompiled::Module)
}

/// Whether `bytes` were compiled ahead of time for the task engine: such a module does not load into the gc_engine
pub fn precompiled_for_tasks(bytes: &[u8]) -> bool {
	// SAFETY: as in compiled_module
	is_precompiled(bytes) && unsafe { Module::deserialize(&crate::util::gc_engine(), bytes) }.is_err()
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
	let digest = Sha256::new().chain_update(engine_hasher.finish().to_le_bytes()).chain_update(bytes).finalize();
	digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The module at `path`, when there is one wasmtime accepts for `engine`; a hit counts as a use for pruning
fn cached_module(engine: &Engine, path: &Path) -> Option<Module> {
	if !path.exists() {
		return None;
	}
	// SAFETY: deserialize trusts the file to be wasmtime's own output: only this module writes the cache directory,
	// atomically (store_module), and wasmtime still rejects a file of another version or engine configuration
	match unsafe { Module::deserialize_file(engine, path) } {
		Ok(module) => {
			let _ = fs::File::options().write(true).open(path).and_then(|file| file.set_modified(SystemTime::now()));
			Some(module)
		}
		Err(failure) => {
			eprintln!("warning: unusable cached module {} is compiled again: {failure}", path.display());
			None
		}
	}
}

/// Write the compiled module to a private file and rename it into place, so parallel runs never read half a file
fn store_module(directory: &Path, path: &Path, module: &Module) -> Result<()> {
	fs::create_dir_all(directory)?;
	PRUNE_ONCE.call_once(|| prune(directory));
	let staging = path.with_extension(format!("{}-{:?}.tmp", std::process::id(), std::thread::current().id()));
	fs::write(&staging, module.serialize()?)?;
	fs::rename(&staging, path)?;
	Ok(())
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

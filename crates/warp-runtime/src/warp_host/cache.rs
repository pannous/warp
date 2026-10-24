//! What warp-host keeps across runs in its cache folder (WARP_HOST_CACHE, else ~/.cache/warp-host): a compiler module's
//! machine code, and the values a program keeps (os.keep, compiler.warp's lowered runtime words; card pure-warm-start)
use crate::nodes::{self, Node};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use wasmtime::{Engine, Module, Result};

pub fn folder() -> Option<PathBuf> {
	let home = || std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"));
	let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(home)?.join("warp-host");
	std::env::var_os("WARP_HOST_CACHE").map(PathBuf::from).or(Some(cache))
}

/// A short fingerprint of anything hashable
pub fn fingerprint(parts: impl Hash) -> String {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	parts.hash(&mut hasher);
	format!("{:016x}", hasher.finish())
}

/// The fingerprint of a folder's files: their paths within it and their contents, so a cache made from the same lib/
/// elsewhere (a shipped cache, `shipped()`) matches; reading lib/ (~1.7 MB) takes a few ms
pub fn folder_fingerprint(folder: &Path) -> String {
	fn walk(root: &Path, folder: &Path, found: &mut Vec<(PathBuf, Vec<u8>)>) {
		let Ok(entries) = std::fs::read_dir(folder) else { return };
		for entry in entries.flatten() {
			let Ok(kind) = entry.file_type() else { continue };
			let path = entry.path();
			if kind.is_dir() {
				walk(root, &path, found)
			} else if let Ok(bytes) = std::fs::read(&path) {
				found.push((path.strip_prefix(root).unwrap_or(&path).to_path_buf(), bytes))
			}
		}
	}
	let mut found = vec![];
	walk(folder, folder, &mut found);
	found.sort();
	fingerprint(found)
}

/// The folders read for a cached file: the cache, then the shipped one, read only: WARP_HOST_SHIPPED_CACHE, else
/// `<bin>/../share/warp/host-cache` or `<bin>/host-cache` beside warp-host (packaging fills it by one run with
/// WARP_HOST_CACHE pointing there)
fn folders() -> Vec<PathBuf> {
	let beside = std::env::current_exe().ok().and_then(|exe| exe.canonicalize().ok()?.parent().map(Path::to_path_buf));
	let shipped = match std::env::var_os("WARP_HOST_SHIPPED_CACHE") {
		Some(folder) => vec![PathBuf::from(folder)],
		None => {
			beside.map(|bin| vec![bin.join("../share/warp/host-cache"), bin.join("host-cache")]).unwrap_or_default()
		}
	};
	folder().into_iter().chain(shipped).collect()
}

/// The module of `bytes`, its machine code from the cache when an engine of these settings compiled it before
pub fn module(engine: &Engine, bytes: &[u8]) -> Result<Module> {
	let name = fingerprint((bytes, engine.precompile_compatibility_hash())) + ".cwasm";
	// a module this process compiled or loaded already (run_wasm of the same bytes again): Module is a cheap handle
	static LOADED: std::sync::Mutex<Vec<(String, Module)>> = std::sync::Mutex::new(Vec::new());
	let loaded = |module: &Module| LOADED.lock().map(|mut loaded| loaded.push((name.clone(), module.clone())));
	if let Some(module) =
		LOADED.lock().ok().and_then(|loaded| loaded.iter().find(|(n, _)| *n == name).map(|e| e.1.clone()))
	{
		return Ok(module);
	}
	let module = machine_code(engine, bytes, &name)?;
	let _ = loaded(&module);
	Ok(module)
}

fn machine_code(engine: &Engine, bytes: &[u8], name: &str) -> Result<Module> {
	for folder in folders() {
		// SAFETY: the file is machine code a warp-host serialized for an engine of the same settings, named by both;
		// wasmtime checks the version, settings and CPU features again, and a mismatch compiles anew
		if let Ok(module) = unsafe { Module::deserialize_file(engine, folder.join(name)) } {
			return Ok(module);
		}
	}
	let module = Module::new(engine, bytes)?;
	if let (Some(folder), Ok(code)) = (folder(), module.serialize()) {
		write(&folder.join(name), &code);
	}
	Ok(module)
}

/// Whose kept values a run reads and writes: the compiler module and lib/ it compiles with (`key`), and that lib/'s
/// path as the compiler was given it. compiler.warp names its values with the path ("… runtime words of <lib>"), so
/// the path is taken out of the name: a cache made from the same lib/ elsewhere (shipped) matches
pub struct Keeper {
	pub key: String,
	pub library: String,
}

/// The value kept under `name` by the program `keeper` names; None without one
pub fn kept(keeper: &Option<Keeper>, name: &str) -> Option<Node> {
	let file = kept_name(keeper.as_ref()?, name);
	let bytes = folders().into_iter().find_map(|folder| std::fs::read(folder.join(&file)).ok())?;
	nodes::decode(&mut bytes.as_slice())
}

/// Keep `value` for the next run; a value warp-host cannot keep (a function…) is computed again then
pub fn keep(keeper: &Option<Keeper>, name: &str, value: &Node) {
	let mut bytes = vec![];
	if let (Some(file), Some(())) =
		(keeper.as_ref().and_then(|keeper| kept_file(keeper, name)), nodes::encode(value, &mut bytes))
	{
		write(&file, &bytes);
	}
}

fn kept_name(keeper: &Keeper, name: &str) -> String {
	let name = if keeper.library.is_empty() { name.to_string() } else { name.replace(&keeper.library, "<lib>") };
	fingerprint((&keeper.key, name)) + ".kept"
}

fn kept_file(keeper: &Keeper, name: &str) -> Option<PathBuf> {
	Some(folder()?.join(kept_name(keeper, name)))
}

/// Written whole or not at all: to a file of its own first, then renamed; a cache that cannot be written is no cache
fn write(file: &Path, bytes: &[u8]) {
	let partial = file.with_extension(format!("{}.partial", std::process::id()));
	let written = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&partial, bytes));
	if written.and_then(|_| std::fs::rename(&partial, file)).is_err() {
		let _ = std::fs::remove_file(&partial);
	}
}

/// The variable naming the lib/ a compiler run reads its std modules from (lib/compiler/parser.warp run_library)
pub const LIBRARY_VARIABLE: &str = "WARP_LIB";

/// The environment a run sees: this process's, with WARP_LIB the compiler run's lib/ (its keeper's) when it has one,
/// so a compiler reads the std modules there, never from the folder it was built in (notes/compiler_rebuilds.md); any
/// other run sees the environment as it is
pub fn environment(keeper: Option<&Keeper>) -> Vec<(String, String)> {
	let library = keeper.map(|keeper| keeper.library.clone()).filter(|library| !library.is_empty());
	let mut variables: Vec<(String, String)> =
		std::env::vars().filter(|(name, _)| library.is_none() || name != LIBRARY_VARIABLE).collect();
	if let Some(library) = library {
		variables.push((LIBRARY_VARIABLE.to_string(), library));
	}
	variables
}

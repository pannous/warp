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

/// The fingerprint of a folder's files as they are now: their paths, sizes and times of change
pub fn folder_fingerprint(folder: &Path) -> String {
	fn walk(folder: &Path, found: &mut Vec<(PathBuf, u64, Option<std::time::SystemTime>)>) {
		let Ok(entries) = std::fs::read_dir(folder) else { return };
		for entry in entries.flatten() {
			let Ok(metadata) = entry.metadata() else { continue };
			if metadata.is_dir() {
				walk(&entry.path(), found)
			} else {
				found.push((entry.path(), metadata.len(), metadata.modified().ok()))
			}
		}
	}
	let mut found = vec![];
	walk(folder, &mut found);
	found.sort();
	fingerprint(found)
}

/// The module of `bytes`, its machine code from the cache when an engine of these settings compiled it before
pub fn module(engine: &Engine, bytes: &[u8]) -> Result<Module> {
	let Some(folder) = folder() else { return Module::new(engine, bytes) };
	let file = folder.join(fingerprint((bytes, engine.precompile_compatibility_hash())) + ".cwasm");
	// SAFETY: the file is machine code this program serialized for an engine of the same settings, named by both
	if let Ok(module) = unsafe { Module::deserialize_file(engine, &file) } {
		return Ok(module);
	}
	let module = Module::new(engine, bytes)?;
	if let Ok(code) = module.serialize() {
		write(&file, &code);
	}
	Ok(module)
}

/// The value kept under `name` by the program `keeper` names; None without one
pub fn kept(keeper: &Option<String>, name: &str) -> Option<Node> {
	let bytes = std::fs::read(kept_file(keeper.as_ref()?, name)?).ok()?;
	nodes::decode(&mut bytes.as_slice())
}

/// Keep `value` for the next run; a value warp-host cannot keep (a function…) is computed again then
pub fn keep(keeper: &Option<String>, name: &str, value: &Node) {
	let mut bytes = vec![];
	if let (Some(file), Some(())) = (keeper.as_ref().and_then(|keeper| kept_file(keeper, name)), nodes::encode(value, &mut bytes)) {
		write(&file, &bytes);
	}
}

fn kept_file(keeper: &str, name: &str) -> Option<PathBuf> {
	Some(folder()?.join(fingerprint((keeper, name)) + ".kept"))
}

/// Written whole or not at all: to a file of its own first, then renamed; a cache that cannot be written is no cache
fn write(file: &Path, bytes: &[u8]) {
	let partial = file.with_extension(format!("{}.partial", std::process::id()));
	let written = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&partial, bytes));
	if written.and_then(|_| std::fs::rename(&partial, file)).is_err() {
		let _ = std::fs::remove_file(&partial);
	}
}

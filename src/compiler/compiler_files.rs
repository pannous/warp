//! The compiler pair's files and keys, as the harness needs them without the Rust compiler (card rust-bootstrap): where
//! lib/ is, the inputs hash of a compiler pair (the shared store's key), the keys of values compiler.warp keeps
//! (os.keep_of), the module of the bytes compiler.wasm made, and the JSON views it writes read back
use crate::node::Node;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The settings of the Rust compiler's self-hosted lanes, kept by name so a compiler pair's key still names them
pub(crate) const SWITCH: &str = "WARP_SELF_HOSTED";
pub(crate) const ANALYSIS_SWITCH: &str = "WARP_SELF_ANALYSIS";
pub(crate) const REFUSALS_FILE: &str = "WARP_EMIT_REFUSALS";
pub(crate) const SURVEY_SETTING: &str = "WARP_EMIT_SURVEY";

/// Where a stage program names lib/, the folder library() resolves
pub(crate) const LIBRARY_MARK: &str = "{lib}";

/// The module the host words are imported from
pub(crate) const HOST_MODULE: &str = "host";

/// The WARP_ settings that change no stage program's compile, so not its kept module either: where lib/ lies is keyed
/// by its contents, hints and windows only change what a run shows, a profile only watches it
#[cfg(feature = "native")]
pub(crate) const SETTINGS_BESIDE_STAGES: [&str; 8] = [
	SWITCH,
	ANALYSIS_SWITCH,
	REFUSALS_FILE,
	SURVEY_SETTING,
	crate::modules::LIBRARY_SETTING,
	crate::errors::diagnostic::HINTS_VARIABLE,
	crate::graphics::paint::NO_WINDOW_VARIABLE,
	warp_runtime::profile::PROFILE_VARIABLE,
];

/// The folder of the warp files a stage program reads: the compiler's and the standard modules it uses (also those
/// runtime.warp names with `use`), resolved once per process (modules::library_folder)
pub(crate) fn library() -> &'static std::path::Path {
	static FOLDER: OnceLock<PathBuf> = OnceLock::new();
	FOLDER.get_or_init(crate::modules::library_folder)
}

/// The folder of the compiler's warp files, where runtime.warp's parts are named from
pub(crate) fn compiler_folder() -> PathBuf {
	library().join(crate::modules::COMPILER_FOLDER)
}

/// The module of the bytes the warp compiler made: WASI always, the host when it imports a host word, libm when it
/// imports pow (a float raised to a power)
pub(crate) fn module_of_bytes(bytes: Vec<u8>) -> crate::compiled_module::CompiledModule {
	let needs_host = imports_from(&bytes, HOST_MODULE);
	let needs_ffi = imports_from(&bytes, crate::ffi::LIBM);
	crate::compiled_module::CompiledModule { bytes, needs_host, needs_wasi: true, needs_ffi }
}

/// Does the module import a function of `module`
pub(crate) fn imports_from(bytes: &[u8], module: &str) -> bool {
	wasmparser::Parser::new(0).parse_all(bytes).any(|payload| match payload {
		Ok(wasmparser::Payload::ImportSection(imports)) => {
			imports.into_imports().flatten().any(|import| import.module == module)
		}
		_ => false,
	})
}

/// How a line names a file or module it uses
pub(crate) const USE_LINE: &str = "use ";

/// The files `roots` name with the files their `use` lines name, each file once, breadth first: [file, its text]
pub(crate) fn use_closure(roots: &[PathBuf]) -> std::io::Result<Vec<(PathBuf, String)>> {
	let mut joined: HashSet<PathBuf> = roots.iter().cloned().collect();
	let mut waiting: VecDeque<PathBuf> = roots.iter().cloned().collect();
	let mut files = Vec::new();
	while let Some(file) = waiting.pop_front() {
		let text = std::fs::read_to_string(&file)?;
		let folder = file.parent().unwrap_or(Path::new(".")).to_path_buf();
		for name in text.lines().filter_map(|line| line.strip_prefix(USE_LINE)).map(str::trim) {
			let path = used_file(&folder, name);
			if joined.insert(path.clone()) {
				waiting.push_back(path);
			}
		}
		files.push((file, text));
	}
	Ok(files)
}

/// The file of a `use` line: a quoted path beside the using file, else a module of lib
pub(crate) fn used_file(folder: &Path, name: &str) -> PathBuf {
	match name.strip_prefix('"').and_then(|quoted| quoted.strip_suffix('"')) {
		// one file however it is reached (runtime/extended/../text.warp is runtime/text.warp)
		Some(path) => folder.join(path).canonicalize().unwrap_or_else(|_| folder.join(path)),
		None => library().join(format!("{name}.warp")),
	}
}

/// The cache key of a value the running compiler keeps under `program` (os.keep): its name, which names the compiler
/// keeping it (its module's SHA-256) and the texts the value was made of, the sources warp was built from (WARP_SOURCE,
/// build.rs) and the WARP_ settings, lib/ written as {lib}. No file of lib/ is read: the compiler embeds core and the
/// modules it uses, so its digest stands for them (card kept-value-key). Neither the binary's place nor its build
/// profile: an identical compiler elsewhere (another worktree) reuses the kept values
#[cfg(feature = "native")]
pub(crate) fn stage_key(program: &str) -> Option<String> {
	use sha2::{Digest, Sha256};
	let program = program.replace(&library().display().to_string(), LIBRARY_MARK);
	let mut settings = crate::run::program_cache::warp_settings();
	settings.retain(|(name, _)| !SETTINGS_BESIDE_STAGES.contains(&name.as_str()));
	let key = Sha256::digest(format!("{}\n{settings:?}\n{program}\n", env!("WARP_SOURCE")));
	Some(crate::extensions::hex_text(&key))
}

/// The key of the value `name` that the core files `makers` made (os.keep_of): the sources warp was built from, the
/// WARP_ settings, the name and the makers' files with those their `use` lines reach, so a compiler built from an
/// edit of core elsewhere keeps the value (card speed-module-cache: uses.warp keeps each used file's parse by
/// parser.warp's files); all of the compiler's inputs when a maker cannot be read
#[cfg(feature = "native")]
pub(crate) fn made_by_key(name: &str, makers: &[String]) -> String {
	use sha2::{Digest, Sha256};
	let mut settings = crate::run::program_cache::warp_settings();
	settings.retain(|(name, _)| !SETTINGS_BESIDE_STAGES.contains(&name.as_str()));
	let made = format!("{}\n{settings:?}\nmade by {makers:?} {}\n{name}\n", env!("WARP_SOURCE"), makers_digest(makers));
	crate::extensions::hex_text(&Sha256::digest(made))
}

/// The SHA-256 of the makers' files and those their `use` lines reach, each by its path inside lib/ and its text, once
/// per process for each set of makers
#[cfg(feature = "native")]
fn makers_digest(makers: &[String]) -> String {
	use sha2::{Digest, Sha256};
	use std::collections::HashMap;
	use std::sync::{LazyLock, Mutex};
	static DIGESTS: LazyLock<Mutex<HashMap<Vec<String>, String>>> = LazyLock::new(Mutex::default);
	let mut digests = DIGESTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let digest = digests.entry(makers.to_vec()).or_insert_with(|| {
		let roots: Vec<PathBuf> = makers.iter().map(|maker| compiler_folder().join(maker)).collect();
		let Ok(files) = use_closure(&roots) else { return compiler_inputs_hash() };
		let mut hasher = Sha256::new();
		for (file, text) in files {
			hasher.update(format!("{}\n", file.strip_prefix(library()).unwrap_or(&file).display()));
			hasher.update(text);
		}
		crate::extensions::hex_text(&hasher.finalize())
	});
	digest.clone()
}

/// The std modules whose text the compiler embeds when it is built (`module_source("math")`): its inputs until
/// lib/compiler reads them from the run's lib/ (card compiler-library-source)
#[cfg(feature = "native")]
pub(crate) const EMBEDDED_MODULES: [&str; 5] = ["aliases", "prelude", "math", "list", "regex"];

/// [path inside lib/, contents] of each compiler input: the files below lib/compiler (dot files left out), sorted,
/// then lib/<module>.warp of each module there is
#[cfg(feature = "native")]
pub(crate) fn compiler_input_files(modules: &[&str]) -> Vec<(String, Vec<u8>)> {
	let library = library();
	let relative = |file: &Path| file.strip_prefix(library).unwrap_or(file).display().to_string();
	let mut inputs: Vec<(String, Vec<u8>)> = files_below(library.join(crate::modules::COMPILER_FOLDER))
		.into_iter()
		.filter(|file| !file.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with('.')))
		.map(|file| (relative(&file), std::fs::read(&file).unwrap_or_default()))
		.collect();
	inputs.sort();
	for module in modules {
		let file = library.join(format!("{module}.warp"));
		if let Ok(bytes) = std::fs::read(&file) {
			inputs.push((relative(&file), bytes));
		}
	}
	inputs
}

/// The inputs hash of the compiler pair built from lib/ by this binary, as scripts/compiler-inputs.sh computes it
/// (16 hex digits): lib/compiler, the embedded modules and WARP_SOURCE, not the seed that built it (a builder, as a C
/// compiler's version: scripts/compiler-inputs.sh names it, so a refreshed seed rebuilds, but a run never judges by it,
/// card scripts-rebuild); the key of the shared compiler store
/// (notes/compiler_rebuilds.md)
#[cfg(feature = "native")]
pub fn compiler_inputs_hash() -> String {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(format!("rust {}\n", env!("WARP_SOURCE")));
	for (file, bytes) in compiler_input_files(&EMBEDDED_MODULES) {
		hasher.update(file.as_bytes());
		hasher.update([0u8]);
		hasher.update(bytes);
		hasher.update([0u8]);
	}
	crate::extensions::hex_text(&hasher.finalize())[..16].to_string()
}

/// The files below `folder`, at any depth
#[cfg(feature = "native")]
pub(crate) fn files_below(folder: PathBuf) -> Vec<PathBuf> {
	let entries = std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|entry| entry.path());
	entries.flat_map(|path| if path.is_dir() { files_below(path) } else { vec![path] }).collect()
}

/// The view a JSON value writes: an array a list, null ø
pub(crate) fn view_of_json(json: serde_json::Value) -> Node {
	use serde_json::Value;
	match json {
		Value::Null => Node::Empty,
		Value::Bool(true) => Node::True,
		Value::Bool(false) => Node::False,
		Value::Number(number) => {
			number.as_i64().map_or_else(|| Node::float(number.as_f64().unwrap_or(f64::NAN)), Node::int)
		}
		Value::String(text) => Node::Text(text),
		Value::Array(items) => crate::compiler::node_views::warp_list(items.into_iter().map(view_of_json)),
		Value::Object(object) => crate::node::error(&format!("a view is no object: {}", Value::Object(object))),
	}
}

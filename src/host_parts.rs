//! Which of the playground's host scripts a module needs (card web-bundle): reader.js and host.js always, and the parts
//! of host.js it imports words of (HOST_PARTS), with the parts they need. A built site (src/site.rs) and a Worker
//! (src/deploy.rs) ship their texts; the playground's Deploy (src/web.rs web_worker_bundle) only names them for
//! warp-hosting, so the wasm32 compiler carries the names without the texts.

use crate::host::{FETCH_REPLY, FETCH_START, FOREIGN_CALL, GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER, HOST_LIBRARY, PAGE_PATH, RUN_BLOCK, SIGNAL_SEND, STD_IO, STD_PURE};
use crate::node::Node;
use warp_runtime::host_words::{RANDOM, RANDOM_BELOW, RANDOM_SEED, SIGNAL_AT, SIGNAL_DAILY, SIGNAL_EVERY};

/// A script's file name and text
pub type Script = (&'static str, &'static str);
pub const WASI_LIBRARY: &str = "wasi_snapshot_preview1";
pub const TASK_WORD_PREFIXES: [&str; 3] = ["task_", "channel_", "shared_"];

/// The text of web/playground scripts, joined; empty without the native feature, where only the names are needed
#[cfg(feature = "native")]
macro_rules! script_text {
	($($file:literal),+) => { concat!($(include_str!(concat!("../web/playground/", $file))),+) };
}
#[cfg(not(feature = "native"))]
macro_rules! script_text {
	($($file:literal),+) => { "" };
}

/// The scripts every page loads before the parts of the host
pub const HOST_SCRIPTS: [Script; 2] = [("reader.js", script_text!("reader.js")), ("host.js", script_text!("host.js"))];

/// A part of host.js (its addHostPart): shipped when the module imports a word it gives, with the parts it needs
pub struct HostPart {
	pub script: Script,
	pub gives: fn(module: &str, name: &str) -> bool,
	pub needs: &'static [&'static str],
}

/// The parts of host.js, in load order (host.js HOST_PART_FILES); the tasks and routes parts start with imports.js, which they
/// read their module's imports with. std_pure and std_io are coarse: they carry every std
/// module's words, so a program using json (std_pure) also gets the hashes
pub const HOST_PARTS: [HostPart; 9] = [
	HostPart {
		script: ("host-files.js", script_text!("host-files.js")),
		gives: |module, name| module == HOST_LIBRARY && ["fetch", "fetch_within", "read", STD_IO].contains(&name),
		needs: &[],
	},
	HostPart { script: ("host-hashes.js", script_text!("host-hashes.js")), gives: |module, name| module == HOST_LIBRARY && name == STD_PURE, needs: &[] },
	HostPart {
		script: ("host-tasks.js", script_text!("imports.js", "host-tasks.js")),
		gives: |module, name| module == HOST_LIBRARY && (TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || [FETCH_START, FETCH_REPLY, SIGNAL_SEND].contains(&name)),
		needs: &[],
	},
	HostPart {
		script: ("host-foreign.js", script_text!("host-foreign.js")),
		gives: |module, name| (module == HOST_LIBRARY && name == FOREIGN_CALL) || ![HOST_LIBRARY, WASI_LIBRARY].contains(&module),
		needs: &["host-files.js"],
	},
	HostPart { script: ("host-compiler.js", script_text!("host-compiler.js")), gives: |module, name| module == HOST_LIBRARY && name == RUN_BLOCK, needs: &["host-files.js"] },
	HostPart { script: ("host-routes.js", script_text!("imports.js", "host-routes.js")), gives: |module, name| module == HOST_LIBRARY && name == PAGE_PATH, needs: &[] },
	HostPart { script: ("host-gpu.js", script_text!("host-gpu.js")), gives: |module, name| module == HOST_LIBRARY && [GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER].contains(&name), needs: &["host-tasks.js"] },
	HostPart { script: ("host-timers.js", script_text!("host-timers.js")), gives: |module, name| module == HOST_LIBRARY && [SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT].contains(&name), needs: &[] },
	HostPart { script: ("host-random.js", script_text!("host-random.js")), gives: |module, name| module == HOST_LIBRARY && [RANDOM, RANDOM_BELOW, RANDOM_SEED].contains(&name), needs: &[] },
];

/// host.js with reader.js and the parts of the host a module imports words of, with the parts they need
pub fn host_scripts_of(module: &[u8]) -> Result<Vec<Script>, String> {
	let imports = imports_of(module)?;
	let imported = |part: &HostPart| imports.iter().any(|(module, name)| (part.gives)(module, name));
	let needed: Vec<&str> = HOST_PARTS.iter().filter(|part| imported(part)).flat_map(|part| part.needs.iter().copied().chain([part.script.0])).collect();
	let parts = HOST_PARTS.iter().map(|part| part.script).filter(|(name, _)| needed.contains(name));
	Ok(HOST_SCRIPTS.into_iter().chain(parts).collect())
}

/// The (module, name) of each import of a module
pub fn imports_of(module: &[u8]) -> Result<Vec<(String, String)>, String> {
	let mut imports = Vec::new();
	for payload in wasmparser::Parser::new(0).parse_all(module) {
		if let wasmparser::Payload::ImportSection(section) = payload.map_err(|failure| failure.to_string())? {
			for import in section.into_imports() {
				let import = import.map_err(|failure| failure.to_string())?;
				imports.push((import.module.to_string(), import.name.to_string()));
			}
		}
	}
	Ok(imports)
}

/// Does the module export a function of this name
pub fn exports(module: &[u8], name: &str) -> bool {
	wasmparser::Parser::new(0).parse_all(module).any(|payload| match payload {
		Ok(wasmparser::Payload::ExportSection(exports)) => exports.into_iter().flatten().any(|export| export.name == name),
		_ => false,
	})
}

/// The module of `code`'s Worker and the host scripts it needs (reader.js, host.js, its parts), which come before
/// cloud-worker.js in worker.js; warp-hosting (web/hosting) takes the module and the scripts' names
pub fn worker_parts(code: &str) -> Result<(Vec<u8>, Vec<Script>), String> {
	// the reflection getters the JavaScript host reads values with
	let module = crate::pipeline::for_any_host(|| crate::pipeline::compile(code)).map_err(|failure| format!("nothing to compile: {}", with_excerpt(code, message_of(&failure))))?;
	if !exports(&module.bytes, crate::lowering::serve::PAGE_SUBMITTED) {
		return Err("the program answers no request: give it a route, `get \"/\" { \"hello\" }`".into());
	}
	let scripts = host_scripts_of(&module.bytes)?;
	Ok((module.bytes, scripts))
}

/// The text of an error, else the value written out
pub(crate) fn message_of(failure: &Node) -> String {
	match failure.drop_meta() {
		Node::Error(message) => match message.drop_meta() {
			Node::Text(text) => text.clone(),
			other => other.serialize(),
		},
		other => other.serialize(),
	}
}

/// A failure's message and, when it names a position, the source line there
pub(crate) fn with_excerpt(code: &str, message: String) -> String {
	let excerpt = crate::diagnostic::message_position(&message).and_then(|(line, column)| crate::diagnostic::excerpt(code, line, column));
	[message].into_iter().chain(excerpt).collect::<Vec<_>>().join("\n")
}

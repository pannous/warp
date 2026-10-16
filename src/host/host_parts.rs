//! Which of the playground's host scripts a module needs (card web-bundle): reader.js and host.js always, and the parts
//! of host.js it imports words of (HOST_PARTS), with the parts they need. A built site (src/web/site.rs) and a Worker
//! (src/web/deploy.rs) ship their texts; the playground's Deploy (src/web/mod.rs web_worker_bundle) only names them for
//! warp-hosting, so the wasm32 compiler carries the names without the texts.

use crate::host::page_parts::{page_layout, PageLayout};
use crate::host::{
	ANY_STD_MODULE, CLIPBOARD_TEXT, DRAW_SHADER, DRAW_SHAPE, DRAW_SHOW, FETCH_REPLY, FETCH_START, FOREIGN_CALL,
	GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER, HOST_LIBRARY, PAGE_PATH, PAINT,
	PARSE_CODE, RUN_BLOCK, SAVE_PICTURE, SIGNAL_SEND, SOUND, STD_IO, STD_MODULES_SECTION, STD_MODULES_SEPARATOR,
	STD_PURE, TEXT_COVERAGE,
};
use crate::node::Node;
use warp_runtime::host_words::{RANDOM, RANDOM_BELOW, RANDOM_SEED, SIGNAL_LISTEN};

/// A script's file name and text
pub type Script = (&'static str, &'static str);
pub const WASI_LIBRARY: &str = "wasi_snapshot_preview1";
pub const TASK_WORD_PREFIXES: [&str; 3] = ["task_", "channel_", "shared_"];
/// The words of WebGPU, which answers asynchronously: their jobs run on a task Worker (host-gpu.js)
pub const GPU_WORDS: [&str; 5] = [GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER];
/// A deployed site runs the program in the visitor's browser, where server routes and functions have no server (card
/// program-server)
const SERVER_AND_BROWSER: &str = "this program uses the browser (it deploys as a site) and has server routes or \
	server functions, which a deployed site cannot answer yet (card program-server): deploy the server part as a \
	program of its own, or run both with `warp serve`";

/// The text of web/playground scripts, joined; empty without the native feature, where only the names are needed
#[cfg(feature = "native")]
macro_rules! script_text {
	($($file:literal),+) => { concat!($(include_str!(concat!("../../web/playground/", $file))),+) };
}
#[cfg(not(feature = "native"))]
macro_rules! script_text {
	($($file:literal),+) => {
		""
	};
}
pub(crate) use script_text;

/// The scripts every page loads before the parts of the host
pub const HOST_SCRIPTS: [Script; 2] = [("reader.js", script_text!("reader.js")), ("host.js", script_text!("host.js"))];

/// A part of host.js (its addHostPart): shipped when the module imports a word it gives, with the parts it needs
pub struct HostPart {
	pub script: Script,
	pub gives: fn(module: &str, name: &str) -> bool,
	pub needs: &'static [&'static str],
}

/// The parts of host.js, in load order (host.js HOST_PART_FILES); the tasks and routes parts start with imports.js, which they
/// read their module's imports with; the tasks, foreign and compiler parts with exact.js, the exact numbers they carry; the files and hashes parts with std.js, the std words. std_pure and std_io carry every std
/// module's words: the std modules a program calls (STD_MODULES_SECTION) choose the parts of their adapters, as if
/// imported from STD_MODULES_SECTION (std_module), so a page keeping values gets no sound code. The pure modules share
/// one part: a program using json also gets the hashes
pub const HOST_PARTS: [HostPart; 11] = [
	HostPart {
		script: ("host-files.js", script_text!("std.js", "host-files.js")),
		gives: |module, name| module == HOST_LIBRARY && ["fetch", "fetch_within", "read", STD_IO].contains(&name),
		needs: &[],
	},
	HostPart {
		script: ("host-sound.js", script_text!("host-sound.js")),
		gives: |module, name| (module == HOST_LIBRARY && name == SOUND) || std_module(module, name, &["sound", "midi"]),
		needs: &["host-files.js"],
	},
	HostPart {
		script: ("host-hashes.js", script_text!("std.js", "host-hashes.js")),
		gives: |module, name| module == HOST_LIBRARY && name == STD_PURE,
		needs: &[],
	},
	HostPart {
		script: ("host-tasks.js", script_text!("imports.js", "exact.js", "host-tasks.js")),
		gives: |module, name| {
			module == HOST_LIBRARY
				&& (TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
					|| [FETCH_START, FETCH_REPLY, SIGNAL_SEND, CLIPBOARD_TEXT].contains(&name))
		},
		needs: &[],
	},
	HostPart {
		script: ("host-foreign.js", script_text!("exact.js", "js-members.js", "host-foreign.js")),
		gives: |module, name| {
			(module == HOST_LIBRARY && name == FOREIGN_CALL)
				|| ![HOST_LIBRARY, WASI_LIBRARY, STD_MODULES_SECTION].contains(&module)
		},
		needs: &["host-files.js"],
	},
	HostPart {
		script: ("host-compiler.js", script_text!("exact.js", "host-compiler.js")),
		gives: |module, name| module == HOST_LIBRARY && [RUN_BLOCK, PARSE_CODE].contains(&name),
		needs: &["host-files.js"],
	},
	HostPart {
		script: ("host-routes.js", script_text!("imports.js", "host-routes.js")),
		gives: |module, name| module == HOST_LIBRARY && name == PAGE_PATH,
		needs: &[],
	},
	// paint may be given a shader (P234), which renders on the GPU (host.js paint, holder.gpuRendered); use draw's
	// GPU words come with paint, as lib/draw.warp imports both
	HostPart {
		script: ("host-gpu.js", script_text!("host-gpu.js")),
		gives: |module, name| {
			module == HOST_LIBRARY
				&& (GPU_WORDS.contains(&name) || [PAINT, DRAW_SHAPE, DRAW_SHOW, DRAW_SHADER].contains(&name))
		},
		needs: &["host-tasks.js", "host-media.js"],
	},
	HostPart {
		script: ("host-timers.js", script_text!("host-timers.js")),
		gives: |module, name| module == HOST_LIBRARY && name == SIGNAL_LISTEN,
		needs: &[],
	},
	HostPart {
		script: ("host-random.js", script_text!("host-random.js")),
		gives: |module, name| module == HOST_LIBRARY && [RANDOM, RANDOM_BELOW, RANDOM_SEED].contains(&name),
		needs: &[],
	},
	// sound_samples is a word of host-sound.js, which reads its samples with host-media.js intsOfList
	HostPart {
		script: ("host-media.js", script_text!("host-media.js")),
		gives: |module, name| module == HOST_LIBRARY && [PAINT, SAVE_PICTURE, TEXT_COVERAGE, SOUND].contains(&name),
		needs: &[],
	},
];

/// host.js with reader.js and the parts of the host a module imports words of, with the parts they need
pub fn host_scripts_of(module: &[u8]) -> Result<Vec<Script>, String> {
	let std_imports = std_modules_of(module)?.into_iter().map(|name| (STD_MODULES_SECTION.to_string(), name));
	let imports: Vec<(String, String)> = imports_of(module)?.into_iter().chain(std_imports).collect();
	let imported = |part: &HostPart| imports.iter().any(|(module, name)| (part.gives)(module, name));
	let needed: Vec<&str> = HOST_PARTS
		.iter()
		.filter(|part| imported(part))
		.flat_map(|part| part.needs.iter().copied().chain([part.script.0]))
		.collect();
	let parts = HOST_PARTS.iter().map(|part| part.script).filter(|(name, _)| needed.contains(name));
	Ok(HOST_SCRIPTS.into_iter().chain(parts).collect())
}

/// Is (module, name) one of these std modules, as host_scripts_of gives the modules a program calls
fn std_module(module: &str, name: &str, adapters: &[&str]) -> bool {
	module == STD_MODULES_SECTION && (name == ANY_STD_MODULE || adapters.contains(&name))
}

/// The std modules a module calls through std_pure / std_io (its STD_MODULES_SECTION), ANY_STD_MODULE for one it
/// names at run time
pub fn std_modules_of(module: &[u8]) -> Result<Vec<String>, String> {
	section_names(module, STD_MODULES_SECTION, STD_MODULES_SEPARATOR)
}

/// The names a custom section of the module lists, joined by the separator; none without that section
pub fn section_names(module: &[u8], section_name: &str, separator: &str) -> Result<Vec<String>, String> {
	for payload in wasmparser::Parser::new(0).parse_all(module) {
		if let wasmparser::Payload::CustomSection(section) = payload.map_err(|failure| failure.to_string())? {
			if section.name() == section_name {
				let names = std::str::from_utf8(section.data()).map_err(|failure| failure.to_string())?;
				return Ok(names.split(separator).map(str::to_string).collect());
			}
		}
	}
	Ok(Vec::new())
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
		Ok(wasmparser::Payload::ExportSection(exports)) => {
			exports.into_iter().flatten().any(|export| export.name == name)
		}
		_ => false,
	})
}

/// What Deploy makes of a program (notes/hosting.md "Browser programs")
pub enum Deployable {
	/// A Worker answering the program's routes: its module and the host scripts that come before cloud-worker.js
	Server { module: Vec<u8>, scripts: Vec<Script> },
	/// A program using the browser (paint, the GPU, sound, page events; card deploy-browser-site): a Worker serving
	/// its site (cloud-site.js), whose page runs the program in the visitor's browser
	Site { module: Vec<u8>, layout: PageLayout },
}

impl Deployable {
	pub fn module(&self) -> &[u8] {
		match self {
			Deployable::Server { module, .. } | Deployable::Site { module, .. } => module,
		}
	}

	/// The scripts warp-hosting composes the Worker of, by name, as /deploy's query: `scripts=` before cloud-worker.js,
	/// or a site's `site=` page scripts, `worker=` scripts of the program's Worker and `files=` every file shipped
	pub fn hosting_query(&self) -> String {
		let names = |scripts: &[Script]| scripts.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(",");
		match self {
			Deployable::Server { scripts, .. } => format!("scripts={}", names(scripts)),
			Deployable::Site { layout, .. } => format!(
				"site={}&worker={}&files={}",
				names(&layout.scripts),
				names(&layout.worker_scripts),
				names(&layout.shipped())
			),
		}
	}
}

/// What Deploy makes of `code`: its site when it uses the browser, else the Worker answering its routes
pub fn deployable(code: &str) -> Result<Deployable, String> {
	let compiled = |build: &dyn Fn() -> Result<crate::pipeline::CompiledModule, Node>| {
		build().map(|module| module.bytes).map_err(|failure| compile_failure(code, &failure))
	};
	// the reflection getters the JavaScript host reads values with
	let module = compiled(&|| {
		crate::pipeline::for_hosting(|| crate::pipeline::for_any_host(|| crate::pipeline::compile(code)))
	})?;
	if uses_the_browser(&module)? {
		if crate::serve::has_server_parts(code) {
			return Err(SERVER_AND_BROWSER.into());
		}
		let module = compiled(&|| crate::pipeline::for_a_page(|| crate::pipeline::compile(code)))?;
		let layout = page_layout(&module, false)?;
		return Ok(Deployable::Site { module, layout });
	}
	if !exports(&module, crate::lowering::serve::PAGE_SUBMITTED) {
		return Err("the program answers no request: give it a route, `get \"/\" { \"hello\" }`".into());
	}
	let scripts = host_scripts_of(&module)?;
	Ok(Deployable::Server { module, scripts })
}

/// Does the module need a browser page (a canvas, the GPU, sound, page events like `on frame`, the clipboard), which a
/// Worker answering requests does not have
fn uses_the_browser(module: &[u8]) -> Result<bool, String> {
	let browser_word = |name: &str| {
		[PAINT, SAVE_PICTURE, DRAW_SHAPE, DRAW_SHOW, DRAW_SHADER, SOUND, TEXT_COVERAGE, CLIPBOARD_TEXT, SIGNAL_LISTEN]
			.contains(&name)
			|| GPU_WORDS.contains(&name)
	};
	Ok(imports_of(module)?.iter().any(|(library, name)| library == HOST_LIBRARY && browser_word(name)))
}

/// Why `code` gave no module: its error with the excerpt, as the CLI says it; a program whose value is all it has
/// (no error) has nothing to compile
pub(crate) fn compile_failure(code: &str, failure: &Node) -> String {
	let message = with_excerpt(code, message_of(failure));
	match failure.drop_meta() {
		Node::Error(_) => message,
		_ => format!("nothing to compile: {message}"),
	}
}

/// The text of an error, else the value written out
pub(crate) fn message_of(failure: &Node) -> String {
	match failure.drop_meta() {
		Node::Error(message) => super::text_or_written(message.drop_meta().clone()),
		other => other.serialize(),
	}
}

/// A failure's message and, when it names a position, the source line there (a module's, when it names the module's file)
pub(crate) fn with_excerpt(code: &str, message: String) -> String {
	let excerpt = match crate::errors::diagnostic::message_file_position(&message) {
		Some((file, line, column)) => std::fs::read_to_string(file)
			.ok()
			.and_then(|module| crate::errors::diagnostic::excerpt(&module, line, column)),
		None => crate::errors::diagnostic::message_position(&message)
			.and_then(|(line, column)| crate::errors::diagnostic::excerpt(code, line, column)),
	};
	[message].into_iter().chain(excerpt).collect::<Vec<_>>().join("\n")
}

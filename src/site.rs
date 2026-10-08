//! `warp build --site app.warp` (card web-ssr, notes/web_framework.md "Built sites"): a directory a static web server
//! serves as it is. index.html holds the HTML of what the program shows, rendered at build time by the program itself
//! (its export page·html, lowering/page_html.rs, run natively after main), so the page reads without JavaScript; app.wasm is the program, which the loader (web/playground/site.js)
//! runs in the page to hydrate it: the DOM stays, the handlers of its elements run, and what they change is shown anew.
//! The page loads the playground's own reader.js, host.js and markup.js, carried in the warp binary, and the parts of
//! host.js its module imports words of (HOST_PARTS, card web-bundle).

use crate::host::{FETCH_REPLY, FETCH_START, FOREIGN_CALL, GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER, HOST_LIBRARY, PAGE_PATH, RUN_BLOCK, SIGNAL_SEND, STD_IO, STD_PURE};
use crate::node::Node;
use warp_runtime::host_words::{RANDOM, RANDOM_BELOW, RANDOM_SEED, SIGNAL_AT, SIGNAL_DAILY, SIGNAL_EVERY};
use std::path::{Path, PathBuf};

const PAGE_FILE: &str = "index.html";
const MODULE_FILE: &str = "app.wasm";
/// The scripts every page loads before the parts of the host, with their text
const HOST_SCRIPTS: [Script; 2] = [("reader.js", include_str!("../web/playground/reader.js")), ("host.js", include_str!("../web/playground/host.js"))];
/// The scripts every page loads after them
const PAGE_SCRIPTS: [Script; 2] = [("markup.js", include_str!("../web/playground/markup.js")), ("site.js", include_str!("../web/playground/site.js"))];
/// The part of markup.js for elements with a CSS transition, after it: only a module that names a transition has them
const TRANSITIONS_SCRIPT: Script = ("markup-transitions.js", include_str!("../web/playground/markup-transitions.js"));
const TRANSITION_WORD: &[u8] = b"transition";
/// A page whose program runs in a Worker (card site-worker) loads this before them, and site.js hands over to it
const THREAD_SCRIPT: Script = ("site-thread.js", include_str!("../web/playground/site-thread.js"));
/// What such a site ships besides: the program's Worker, the task Workers it starts, and the service worker that gives a
/// static host's page the cross-origin isolation shared memory needs
const WORKER_FILES: [Script; 3] = [
	("site-worker.js", include_str!("../web/playground/site-worker.js")),
	("task-worker.js", include_str!("../web/playground/task-worker.js")),
	("coi-serviceworker.js", include_str!("../web/playground/coi-serviceworker.js")),
];
/// The root's attribute listing the scripts of the program's Worker (site-thread.js WORKER_ATTRIBUTE)
const WORKER_ATTRIBUTE: &str = "data-warp-worker";
const LIST_SEPARATOR: &str = ",";
type Script = (&'static str, &'static str);
const WASI_LIBRARY: &str = "wasi_snapshot_preview1";
const TASK_WORD_PREFIXES: [&str; 3] = ["task_", "channel_", "shared_"];

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
		script: ("host-files.js", include_str!("../web/playground/host-files.js")),
		gives: |module, name| module == HOST_LIBRARY && ["fetch", "fetch_within", "read", STD_IO].contains(&name),
		needs: &[],
	},
	HostPart { script: ("host-hashes.js", include_str!("../web/playground/host-hashes.js")), gives: |module, name| module == HOST_LIBRARY && name == STD_PURE, needs: &[] },
	HostPart {
		script: ("host-tasks.js", concat!(include_str!("../web/playground/imports.js"), include_str!("../web/playground/host-tasks.js"))),
		gives: |module, name| module == HOST_LIBRARY && (TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || [FETCH_START, FETCH_REPLY, SIGNAL_SEND].contains(&name)),
		needs: &[],
	},
	HostPart {
		script: ("host-foreign.js", include_str!("../web/playground/host-foreign.js")),
		gives: |module, name| (module == HOST_LIBRARY && name == FOREIGN_CALL) || ![HOST_LIBRARY, WASI_LIBRARY].contains(&module),
		needs: &["host-files.js"],
	},
	HostPart { script: ("host-compiler.js", include_str!("../web/playground/host-compiler.js")), gives: |module, name| module == HOST_LIBRARY && name == RUN_BLOCK, needs: &["host-files.js"] },
	HostPart { script: ("host-routes.js", concat!(include_str!("../web/playground/imports.js"), include_str!("../web/playground/host-routes.js"))), gives: |module, name| module == HOST_LIBRARY && name == PAGE_PATH, needs: &[] },
	HostPart { script: ("host-gpu.js", include_str!("../web/playground/host-gpu.js")), gives: |module, name| module == HOST_LIBRARY && [GPU_COMPUTE, GPU_COMPUTE_LINEAR, GPU_MAP_LINEAR, GPU_REDUCE_LINEAR, GPU_RENDER].contains(&name), needs: &["host-tasks.js"] },
	HostPart { script: ("host-timers.js", include_str!("../web/playground/host-timers.js")), gives: |module, name| module == HOST_LIBRARY && [SIGNAL_EVERY, SIGNAL_DAILY, SIGNAL_AT].contains(&name), needs: &[] },
	HostPart { script: ("host-random.js", include_str!("../web/playground/host-random.js")), gives: |module, name| module == HOST_LIBRARY && [RANDOM, RANDOM_BELOW, RANDOM_SEED].contains(&name), needs: &[] },
];
/// The parts the page keeps when the program runs in a Worker: host-routes.js follows links and the back button and
/// moves the focus, which only the page can (site-thread.js)
const PAGE_SIDE_PARTS: [&str; 1] = ["host-routes.js"];
const LINE_COMMENT: &str = "//";
/// The element holding the program's markup (site.js SITE_ROOT)
const ROOT_ID: &str = "warp-root";
const PAGE_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{title}}</title>
{{base}}</head>
<body>
<div id="{{root}}"{{worker}}>{{body}}</div>
{{scripts}}</body>
</html>
"#;

/// `warp dev` adds the script asking the dev server for new builds (src/dev_server.rs)
const DEV_SCRIPT: (&str, &str) = ("dev.js", include_str!("../web/playground/dev.js"));

/// The content types of a site's files, by their extension
const CONTENT_TYPES: [(&str, &str); 3] = [("html", "text/html; charset=utf-8"), ("js", "text/javascript; charset=utf-8"), ("wasm", "application/wasm")];
const OTHER_CONTENT: &str = "application/octet-stream";
const INDEX_PATH: &str = "/";
/// The page of a program with routes for any other path (card single-page): a static host such as GitHub Pages serves
/// 404.html for a path it has no file of, `warp dev` and `serve` do the same; the page's router shows that path's route
const ROUTED_PAGE_FILE: &str = "404.html";
/// Where the page finds the site's files: index.html beside them, 404.html at any depth from the site's root
const BESIDE: &str = "";
const SITE_ROOT: &str = "/";

/// What `warp build --site` wrote
pub struct BuiltSite {
	pub directory: PathBuf,
	pub files: Vec<String>,
}

/// A file of a site: its name and content
pub type SiteFile = (String, Vec<u8>);

/// The site of the program `code` in `directory`, titled `title`: the page, the module and the scripts
pub fn build(code: &str, title: &str, directory: &Path) -> Result<BuiltSite, String> {
	let files = files(code, title, false)?;
	std::fs::create_dir_all(directory).map_err(|failure| format!("cannot make {}: {failure}", directory.display()))?;
	for (name, bytes) in &files {
		std::fs::write(directory.join(name), bytes).map_err(|failure| format!("cannot write {name}: {failure}"))?;
	}
	Ok(BuiltSite { directory: directory.to_path_buf(), files: files.into_iter().map(|(name, _)| name).collect() })
}

/// The files of the site of `code`; `dev` adds dev.js. A failure names its position with the source line
pub fn files(code: &str, title: &str, dev: bool) -> Result<Vec<SiteFile>, String> {
	site_files(code, title, dev)?.ok_or_else(|| format!("the program shows no page: it exports no {}", crate::page_html::PAGE_HTML))
}

/// The files of the site a program serving its page serves (src/web_server.rs), none when its last line shows nothing
pub fn served_files(code: &str, title: &str) -> Result<Option<Vec<SiteFile>>, String> {
	site_files(code, title, false)
}

fn site_files(code: &str, title: &str, dev: bool) -> Result<Option<Vec<SiteFile>>, String> {
	let compile = |build: &dyn Fn() -> Result<crate::pipeline::CompiledModule, Node>| {
		let module = crate::pipeline::for_a_page(|| if dev { crate::pipeline::for_dev(build) } else { build() });
		module.map_err(|value| format!("nothing to compile: {}", with_excerpt(code, message_of(&value))))
	};
	let rendering = compile(&|| crate::pipeline::prerendering(|| crate::pipeline::compile(code)))?;
	if !exports(&rendering.bytes, crate::page_html::PAGE_HTML) {
		return Ok(None);
	}
	let read_after_main = |name: &str| {
		let imports = crate::wasm_reader::Imports { host: rendering.needs_host, wasi: rendering.needs_wasi, ffi: rendering.needs_ffi };
		crate::wasm_reader::read_export_after_main(&rendering.bytes, imports, name).map_err(|failure| format!("the program failed at build time: {}", with_excerpt(code, failure.to_string())))
	};
	let rendered = read_after_main(crate::page_html::PAGE_HTML)?;
	// a page calling server functions ships without them, starting from the values they gave here (lowering/serve.rs)
	let module = if exports(&rendering.bytes, crate::serve::RPC_VALUES) {
		let values = match read_after_main(crate::serve::RPC_VALUES)?.drop_meta() {
			Node::List(values, _, _) => values.clone(),
			single => vec![single.clone()],
		};
		compile(&|| crate::pipeline::with_server_values(values.clone(), || crate::pipeline::compile(code)))?
	} else {
		rendering
	};
	let Node::Text(html) = rendered.drop_meta() else {
		return Err(format!("{} gave no text: {}", crate::page_html::PAGE_HTML, rendered.serialize()));
	};
	let host_scripts = host_scripts_of(&module.bytes)?;
	let dev_script = dev.then_some(DEV_SCRIPT);
	let (scripts, worker_scripts): (Vec<Script>, Vec<Script>) = if runs_in_a_worker(&imports_of(&module.bytes)?) {
		let page_parts = host_scripts.iter().filter(|(name, _)| PAGE_SIDE_PARTS.contains(name)).copied();
		([THREAD_SCRIPT].into_iter().chain(page_parts).chain(page_scripts_of(&module.bytes)).chain(dev_script).collect(), host_scripts)
	} else {
		(host_scripts.into_iter().chain(page_scripts_of(&module.bytes)).chain(dev_script).collect(), vec![])
	};
	let page = |root: &str| page(title, html, &scripts, &worker_scripts, root).into_bytes();
	let routed = exports(&module.bytes, crate::routes::PAGE_ROUTES);
	// each route's own functions in a module the page loads when it shows the route; a dev page reloads whole anyway
	let split = if dev { None } else { crate::route_split::split_by_route(&module.bytes)? };
	let (primary, route_modules) = match split {
		Some(split) => (split.primary, split.routes),
		None => (module.bytes, vec![]),
	};
	let mut files = vec![(PAGE_FILE.to_string(), page(BESIDE)), (MODULE_FILE.to_string(), primary)];
	if routed {
		files.push((ROUTED_PAGE_FILE.to_string(), page(SITE_ROOT)));
	}
	files.extend(route_modules);
	let worker_files = if worker_scripts.is_empty() { &[][..] } else { &WORKER_FILES[..] };
	let shipped = scripts.iter().chain(worker_scripts.iter().filter(|script| !scripts.contains(script))).chain(worker_files);
	files.extend(shipped.map(|(name, text)| (name.to_string(), compacted(text).into_bytes())));
	Ok(Some(files))
}

/// The file of a site a request path names: "/" is the page, "/app.wasm" the module, …, any other path the page of a
/// program with routes (404.html)
pub fn file_at<'a>(files: &'a [SiteFile], path: &str) -> Option<&'a SiteFile> {
	let name = if path == INDEX_PATH { PAGE_FILE } else { path.trim_start_matches('/') };
	let named = |wanted: &str| files.iter().find(|(file, _)| file == wanted);
	named(name).or_else(|| named(ROUTED_PAGE_FILE))
}

/// The content type of a site's file, by its extension
pub fn content_type(name: &str) -> &'static str {
	let extension = Path::new(name).extension().and_then(|extension| extension.to_str()).unwrap_or_default();
	CONTENT_TYPES.iter().find(|(known, _)| *known == extension).map_or(OTHER_CONTENT, |(_, content_type)| content_type)
}

/// Does the module export a function of this name
fn exports(module: &[u8], name: &str) -> bool {
	wasmparser::Parser::new(0).parse_all(module).any(|payload| match payload {
		Ok(wasmparser::Payload::ExportSection(exports)) => exports.into_iter().flatten().any(|export| export.name == name),
		_ => false,
	})
}

/// The page `warp dev` shows before any build succeeded: only dev.js, which shows the failure
pub fn dev_shell(title: &str) -> Vec<SiteFile> {
	let page = page(title, "", &[DEV_SCRIPT], &[], BESIDE);
	vec![(PAGE_FILE.to_string(), page.into_bytes()), (DEV_SCRIPT.0.to_string(), DEV_SCRIPT.1.as_bytes().to_vec())]
}

/// The scripts of the page of a module, in load order: the parts of the host it imports words of, with the parts they
/// need, and markup-transitions.js when it names a transition; `dev` adds dev.js
pub fn scripts_of(module: &[u8], dev: bool) -> Result<Vec<Script>, String> {
	Ok(host_scripts_of(module)?.into_iter().chain(page_scripts_of(module)).chain(dev.then_some(DEV_SCRIPT)).collect())
}

/// host.js with reader.js and the parts of the host a module imports words of, with the parts they need
fn host_scripts_of(module: &[u8]) -> Result<Vec<Script>, String> {
	let imports = imports_of(module)?;
	let imported = |part: &HostPart| imports.iter().any(|(module, name)| (part.gives)(module, name));
	let needed: Vec<&str> = HOST_PARTS.iter().filter(|part| imported(part)).flat_map(|part| part.needs.iter().copied().chain([part.script.0])).collect();
	let parts = HOST_PARTS.iter().map(|part| part.script).filter(|(name, _)| needed.contains(name));
	Ok(HOST_SCRIPTS.into_iter().chain(parts).collect())
}

/// markup.js, its transitions part when the module names a transition, and site.js
fn page_scripts_of(module: &[u8]) -> Vec<Script> {
	let [markup, site] = PAGE_SCRIPTS;
	let transitions = module.windows(TRANSITION_WORD.len()).any(|window| window == TRANSITION_WORD).then_some(TRANSITIONS_SCRIPT);
	[markup].into_iter().chain(transitions).chain([site]).collect()
}

/// A module that starts tasks, uses channels or shared memory runs in a Worker, where a blocking `await` may wait and
/// its tasks run together (card site-worker); a plain page stays on the page's thread. Routes go along: the page sends
/// the Worker the path of each link followed (site-thread.js)
fn runs_in_a_worker(imports: &[(String, String)]) -> bool {
	imports.iter().any(|(module, name)| module == HOST_LIBRARY && TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix)))
}

/// The (module, name) of each import of a module
fn imports_of(module: &[u8]) -> Result<Vec<(String, String)>, String> {
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

/// The text of an error, else the value written out
fn message_of(failure: &Node) -> String {
	match failure.drop_meta() {
		Node::Error(message) => match message.drop_meta() {
			Node::Text(text) => text.clone(),
			other => other.serialize(),
		},
		other => other.serialize(),
	}
}

/// A failure's message and, when it names a position, the source line there
fn with_excerpt(code: &str, message: String) -> String {
	let excerpt = crate::diagnostic::message_position(&message).and_then(|(line, column)| crate::diagnostic::excerpt(code, line, column));
	[message].into_iter().chain(excerpt).collect::<Vec<_>>().join("\n")
}

/// A page served at a deeper path than the site's files finds them, and the module and route modules they load,
/// through its <base> (`root`)
fn page(title: &str, body: &str, scripts: &[Script], worker_scripts: &[Script], root: &str) -> String {
	let scripts: String = scripts.iter().map(|(name, _)| format!("<script src=\"{name}\"></script>\n")).collect();
	let worker_list: Vec<&str> = worker_scripts.iter().map(|(name, _)| *name).collect();
	let worker = if worker_list.is_empty() { String::new() } else { format!(" {WORKER_ATTRIBUTE}=\"{}\"", worker_list.join(LIST_SEPARATOR)) };
	let base = if root == BESIDE { String::new() } else { format!("<base href=\"{root}\">\n") };
	// the program's texts last, so nothing in them is read as a placeholder
	PAGE_TEMPLATE.replace("{{root}}", ROOT_ID).replace("{{scripts}}", &scripts).replace("{{worker}}", &worker).replace("{{base}}", &base).replace("{{title}}", &escaped(title)).replace("{{body}}", body)
}

/// A script without its comment lines, blank lines and indentation (card web-bundle: host.js gzipped 24 → 17 KB); the
/// lines of a template literal stay as written
pub fn compacted(script: &str) -> String {
	let mut in_template = false;
	let mut kept = String::new();
	for line in script.lines() {
		let shown = if in_template { line } else { line.trim_start() };
		if in_template || !(shown.is_empty() || shown.starts_with(LINE_COMMENT)) {
			kept.push_str(shown);
			kept.push('\n');
		}
		in_template ^= line.matches('`').count() % 2 == 1;
	}
	kept
}

fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

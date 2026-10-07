//! `warp build --site app.wasp` (card web-ssr, notes/web_framework.md "Built sites"): a directory a static web server
//! serves as it is. index.html holds the HTML of what the program shows, rendered at build time by the program itself
//! (its export page·html, lowering/page_html.rs, run natively after main), so the page reads without JavaScript; app.wasm is the program, which the loader (web/playground/site.js)
//! runs in the page to hydrate it: the DOM stays, the handlers of its elements run, and what they change is shown anew.
//! The page loads the playground's own reader.js, host.js and markup.js, carried in the warp binary, and the parts of
//! host.js its module imports words of (HOST_PARTS, card web-bundle).

use crate::host::{FETCH_REPLY, FETCH_START, FOREIGN_CALL, HOST_LIBRARY, RUN_BLOCK, SIGNAL_SEND, STD_IO, STD_PURE};
use crate::node::Node;
use std::path::{Path, PathBuf};

const PAGE_FILE: &str = "index.html";
const MODULE_FILE: &str = "app.wasm";
/// The scripts every page loads before the parts of the host, with their text
const HOST_SCRIPTS: [Script; 2] = [("reader.js", include_str!("../web/playground/reader.js")), ("host.js", include_str!("../web/playground/host.js"))];
/// The scripts every page loads after them
const PAGE_SCRIPTS: [Script; 2] = [("markup.js", include_str!("../web/playground/markup.js")), ("site.js", include_str!("../web/playground/site.js"))];
type Script = (&'static str, &'static str);
const WASI_LIBRARY: &str = "wasi_snapshot_preview1";
const TASK_WORD_PREFIXES: [&str; 3] = ["task_", "channel_", "shared_"];

/// A part of host.js (its addHostPart): shipped when the module imports a word it gives, with the parts it needs
pub struct HostPart {
	pub script: Script,
	pub gives: fn(module: &str, name: &str) -> bool,
	pub needs: &'static [&'static str],
}

/// The parts of host.js, in load order (host.js HOST_PART_FILES). std_pure and std_io are coarse: they carry every std
/// module's words, so a program using json (std_pure) also gets the hashes
pub const HOST_PARTS: [HostPart; 5] = [
	HostPart {
		script: ("host-files.js", include_str!("../web/playground/host-files.js")),
		gives: |module, name| module == HOST_LIBRARY && ["fetch", "fetch_within", "read", STD_IO].contains(&name),
		needs: &[],
	},
	HostPart { script: ("host-hashes.js", include_str!("../web/playground/host-hashes.js")), gives: |module, name| module == HOST_LIBRARY && name == STD_PURE, needs: &[] },
	HostPart {
		script: ("host-tasks.js", include_str!("../web/playground/host-tasks.js")),
		gives: |module, name| module == HOST_LIBRARY && (TASK_WORD_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || [FETCH_START, FETCH_REPLY, SIGNAL_SEND].contains(&name)),
		needs: &[],
	},
	HostPart {
		script: ("host-foreign.js", include_str!("../web/playground/host-foreign.js")),
		gives: |module, name| (module == HOST_LIBRARY && name == FOREIGN_CALL) || ![HOST_LIBRARY, WASI_LIBRARY].contains(&module),
		needs: &["host-files.js"],
	},
	HostPart { script: ("host-compiler.js", include_str!("../web/playground/host-compiler.js")), gives: |module, name| module == HOST_LIBRARY && name == RUN_BLOCK, needs: &["host-files.js"] },
];
const LINE_COMMENT: &str = "//";
/// The element holding the program's markup (site.js SITE_ROOT)
const ROOT_ID: &str = "wasp-root";
const PAGE_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{title}}</title>
</head>
<body>
<div id="{{root}}">{{body}}</div>
{{scripts}}</body>
</html>
"#;

/// `warp dev` adds the script asking the dev server for new builds (src/dev_server.rs)
const DEV_SCRIPT: (&str, &str) = ("dev.js", include_str!("../web/playground/dev.js"));

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
	let compile = || crate::pipeline::for_a_page(|| crate::pipeline::compile(code));
	let module = if dev { crate::pipeline::for_dev(compile) } else { compile() };
	let module = module.map_err(|value| format!("nothing to compile: {}", with_excerpt(code, message_of(&value))))?;
	let imports = crate::wasm_reader::Imports { host: module.needs_host, wasi: module.needs_wasi, ffi: module.needs_ffi };
	let rendered = crate::wasm_reader::read_export_after_main(&module.bytes, imports, crate::page_html::PAGE_HTML)
		.map_err(|failure| format!("the program failed at build time: {}", with_excerpt(code, failure.to_string())))?;
	let Node::Text(html) = rendered.drop_meta() else {
		return Err(format!("{} gave no text: {}", crate::page_html::PAGE_HTML, rendered.serialize()));
	};
	let scripts = scripts_of(&module.bytes, dev)?;
	let page = page(title, html, &scripts);
	let mut files = vec![(PAGE_FILE.to_string(), page.into_bytes()), (MODULE_FILE.to_string(), module.bytes)];
	files.extend(scripts.iter().map(|(name, text)| (name.to_string(), compacted(text).into_bytes())));
	Ok(files)
}

/// The page `warp dev` shows before any build succeeded: only dev.js, which shows the failure
pub fn dev_shell(title: &str) -> Vec<SiteFile> {
	let page = page(title, "", &[DEV_SCRIPT]);
	vec![(PAGE_FILE.to_string(), page.into_bytes()), (DEV_SCRIPT.0.to_string(), DEV_SCRIPT.1.as_bytes().to_vec())]
}

/// The scripts of the page of a module, in load order: the parts of the host it imports words of, with the parts they
/// need; `dev` adds dev.js
pub fn scripts_of(module: &[u8], dev: bool) -> Result<Vec<Script>, String> {
	let imports = imports_of(module)?;
	let imported = |part: &HostPart| imports.iter().any(|(module, name)| (part.gives)(module, name));
	let needed: Vec<&str> = HOST_PARTS.iter().filter(|part| imported(part)).flat_map(|part| part.needs.iter().copied().chain([part.script.0])).collect();
	let parts = HOST_PARTS.iter().map(|part| part.script).filter(|(name, _)| needed.contains(name));
	Ok(HOST_SCRIPTS.into_iter().chain(parts).chain(PAGE_SCRIPTS).chain(dev.then_some(DEV_SCRIPT)).collect())
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

fn page(title: &str, body: &str, scripts: &[(&str, &str)]) -> String {
	let scripts: String = scripts.iter().map(|(name, _)| format!("<script src=\"{name}\"></script>\n")).collect();
	// the program's texts last, so nothing in them is read as a placeholder
	PAGE_TEMPLATE.replace("{{root}}", ROOT_ID).replace("{{scripts}}", &scripts).replace("{{title}}", &escaped(title)).replace("{{body}}", body)
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

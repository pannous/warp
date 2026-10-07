//! `warp build --site app.wasp` (card web-ssr, notes/web_framework.md "Built sites"): a directory a static web server
//! serves as it is. index.html holds the HTML of what the program shows, rendered at build time by the program itself
//! (its export page·html, lowering/page_html.rs, run natively after main), so the page reads without JavaScript; app.wasm is the program, which the loader (web/playground/site.js)
//! runs in the page to hydrate it: the DOM stays, the handlers of its elements run, and what they change is shown anew.
//! The page loads the playground's own reader.js, host.js and markup.js, carried in the warp binary.

use crate::node::Node;
use std::path::{Path, PathBuf};

const PAGE_FILE: &str = "index.html";
const MODULE_FILE: &str = "app.wasm";
/// The scripts of the page, in load order, with their text
const SCRIPTS: [(&str, &str); 4] = [
	("reader.js", include_str!("../web/playground/reader.js")),
	("host.js", include_str!("../web/playground/host.js")),
	("markup.js", include_str!("../web/playground/markup.js")),
	("site.js", include_str!("../web/playground/site.js")),
];
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
	let module = crate::pipeline::for_a_page(|| crate::pipeline::compile(code)).map_err(|value| format!("nothing to compile: {}", with_excerpt(code, message_of(&value))))?;
	let imports = crate::wasm_reader::Imports { host: module.needs_host, wasi: module.needs_wasi, ffi: module.needs_ffi };
	let rendered = crate::wasm_reader::read_export_after_main(&module.bytes, imports, crate::page_html::PAGE_HTML)
		.map_err(|failure| format!("the program failed at build time: {}", with_excerpt(code, failure.to_string())))?;
	let Node::Text(html) = rendered.drop_meta() else {
		return Err(format!("{} gave no text: {}", crate::page_html::PAGE_HTML, rendered.serialize()));
	};
	let scripts = scripts(dev);
	let page = page(title, html, &scripts);
	let mut files = vec![(PAGE_FILE.to_string(), page.into_bytes()), (MODULE_FILE.to_string(), module.bytes)];
	files.extend(scripts.iter().map(|(name, text)| (name.to_string(), text.as_bytes().to_vec())));
	Ok(files)
}

/// The page `warp dev` shows before any build succeeded: only dev.js, which shows the failure
pub fn dev_shell(title: &str) -> Vec<SiteFile> {
	let page = page(title, "", &[DEV_SCRIPT]);
	vec![(PAGE_FILE.to_string(), page.into_bytes()), (DEV_SCRIPT.0.to_string(), DEV_SCRIPT.1.as_bytes().to_vec())]
}

fn scripts(dev: bool) -> Vec<(&'static str, &'static str)> {
	SCRIPTS.into_iter().chain(dev.then_some(DEV_SCRIPT)).collect()
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

fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

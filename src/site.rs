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

/// What `warp build --site` wrote
pub struct BuiltSite {
	pub directory: PathBuf,
	pub files: Vec<String>,
}

/// A file of a site: its name and bytes
pub type SiteFile = (&'static str, Vec<u8>);

/// The site of the program `code` in `directory`, titled `title`: the page, the module and the scripts
pub fn build(code: &str, title: &str, directory: &Path) -> Result<BuiltSite, String> {
	let site = render(code, title)?.ok_or_else(|| format!("the program shows no page: it exports no {}", crate::page_html::PAGE_HTML))?;
	std::fs::create_dir_all(directory).map_err(|failure| format!("cannot make {}: {failure}", directory.display()))?;
	for (name, bytes) in &site {
		std::fs::write(directory.join(name), bytes).map_err(|failure| format!("cannot write {name}: {failure}"))?;
	}
	Ok(BuiltSite { directory: directory.to_path_buf(), files: site.iter().map(|(name, _)| name.to_string()).collect() })
}

/// The files of the site of the program `code` (index.html first), none when its last line shows nothing
pub fn render(code: &str, title: &str) -> Result<Option<Vec<SiteFile>>, String> {
	let module = crate::pipeline::for_a_page(|| crate::pipeline::compile(code)).map_err(|value| format!("nothing to compile: {}", value.serialize()))?;
	if !exports(&module.bytes, crate::page_html::PAGE_HTML) {
		return Ok(None);
	}
	let imports = crate::wasm_reader::Imports { host: module.needs_host, wasi: module.needs_wasi, ffi: module.needs_ffi };
	let rendered = crate::wasm_reader::read_export_after_main(&module.bytes, imports, crate::page_html::PAGE_HTML)
		.map_err(|failure| format!("the program failed at build time: {failure}"))?;
	let Node::Text(html) = rendered.drop_meta() else {
		return Err(format!("{} gave no text: {}", crate::page_html::PAGE_HTML, rendered.serialize()));
	};
	let pages = [(PAGE_FILE, page(title, html).into_bytes()), (MODULE_FILE, module.bytes)];
	Ok(Some(pages.into_iter().chain(SCRIPTS.map(|(name, text)| (name, text.as_bytes().to_vec()))).collect()))
}

/// Does the module export a function of this name
fn exports(module: &[u8], name: &str) -> bool {
	wasmparser::Parser::new(0).parse_all(module).any(|payload| match payload {
		Ok(wasmparser::Payload::ExportSection(exports)) => exports.into_iter().flatten().any(|export| export.name == name),
		_ => false,
	})
}

fn page(title: &str, body: &str) -> String {
	let scripts: String = SCRIPTS.iter().map(|(name, _)| format!("<script src=\"{name}\"></script>\n")).collect();
	// the program's texts last, so nothing in them is read as a placeholder
	PAGE_TEMPLATE.replace("{{root}}", ROOT_ID).replace("{{scripts}}", &scripts).replace("{{title}}", &escaped(title)).replace("{{body}}", body)
}

fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
